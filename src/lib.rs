extern crate git2;
extern crate tempfile;
extern crate toml;

pub mod auto_filters;
pub mod git_utils;
pub mod my_utils;
pub mod symbolic_link;
pub mod user_config;

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use clap::ArgMatches;

use auto_filters::filter_target_dirs;
use my_utils::make_then_check_path;
use user_config::Conf;

#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub enum Action {
    Make,
    Delete,
    Remake,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let action_str = match self {
            Action::Make => "make",
            Action::Delete => "delete",
            Action::Remake => "remake",
        };

        write!(f, "{}", action_str)
    }
}

#[derive(Debug)]
pub struct MainApp {
    pub under_dir: PathBuf,
    pub upper_dir: PathBuf,
    pub target_dirs: Vec<PathBuf>,
    pub verbose_num: u64,
    pub dry_run: bool,
    pub action: Action,
    pub git_url: Option<String>,
}

impl MainApp {
    pub fn new(
        args: ArgMatches,
        config: Result<Conf, Box<dyn Error>>,
    ) -> Result<Self, Box<dyn Error>> {
        let verbose_num = args.occurrences_of("verbose");

        let dry_run = args.is_present("dryrun");

        let initial_target_path: Option<PathBuf> = args
            .value_of("target")
            .map(|val| make_then_check_path(&[val]))
            .unwrap_or(None);

        let config = if let Ok(conf) = config {
            Some(conf)
        } else if initial_target_path.is_none() {
            if let Err(err) = config {
                return Err(Box::from(format!("config and cli args: {}", err)));
            } else {
                None
            }
        } else {
            None
        };

        if verbose_num > 0 && config.is_some() {
            println!("got config");
        }

        let init_upper: Option<PathBuf> = args
            .value_of("upper")
            .map(|val| make_then_check_path(&[val]))
            .unwrap_or(None);

        let action = if args.is_present("delete") {
            Action::Delete
        } else if args.is_present("remake") {
            Action::Remake
        } else {
            Action::Make
        };

        let gurl = args.value_of("git_url").map(|val| val.to_string());

        // if initial_target_path given then use .. as upper_dir
        let upper_dir: PathBuf = if let Some(ini_up) = &initial_target_path {
            ini_up.clone()
        // else try and get upper_dir from config
        } else if let Some(conf) = &config {
            my_utils::make_then_check_path(&[&conf.upper_dir])
                .ok_or("can't get auto upper dir")?
        } else if let Some(upper) = init_upper {
            upper
        } else if let Some(ini_target) = &initial_target_path {
            let mut initial_anc = ini_target.ancestors();
            initial_anc.next();
            initial_anc.next().unwrap().to_path_buf()
        } else {
            return Err(Box::from("somethings fucked up"));
        };

        if verbose_num == 1 {
            println!("got upper dir");
        } else if verbose_num > 1 {
            println!("got upper dir {:?}", upper_dir);
        }

        let under_dir: PathBuf = if let Some(ini_path) = &initial_target_path {
            // upper can be else where
            ini_path.ancestors().next().unwrap().to_owned()
        } else if let Some(conf) = &config {
            upper_dir.join(&conf.under_dir)
        } else {
            return Err(Box::from("no under in config or cli"));
        };

        let target_dirs: Vec<PathBuf> =
            if let Some(ini_target) = &initial_target_path {
                vec![ini_target.clone()]
            } else if let Some(conf) = &config {
                filter_target_dirs(&under_dir, &conf)?
            } else {
                return Err(Box::from("no config for auto"));
            };

        if verbose_num == 1 {
            println!("got target_dirs dir[s]");
        } else if verbose_num > 1 {
            let target_iter: Vec<String> = target_dirs
                .iter()
                .map(|path| path.to_str().unwrap().to_owned())
                .collect();

            let msg_string = my_utils::vec_to_string("", &target_iter);

            println!("got target dir[s] {}", &msg_string);
        }

        let git_url: Option<String> = if let Some(conf) = &config {
            if conf.git_url.is_some() {
                conf.git_url.clone()
            } else {
                gurl
            }
        } else {
            gurl
        };

        Ok(MainApp {
            under_dir,
            upper_dir,
            target_dirs,
            verbose_num,
            dry_run,
            action,
            git_url,
        })
    }

    pub fn verbose_ouput(&self, message: &str, more: Option<&str>) {
        if self.verbose_num == 1 && !message.is_empty() {
            println!("{}", message);
        } else if self.verbose_num == 2 {
            println!("{}{}", message, more.unwrap_or(""));
        }
    }
}
