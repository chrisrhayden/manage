/// get the user config
///
/// make sure the git repo hase been clone
/// and both server and local copy are up to date
///
/// then make all the needed symlinks

#[macro_use]
extern crate clap;
use clap::App;

use std::error::Error;

use manage::{
    git_utils::manage_git,
    my_utils::make_then_check_path,
    symbolic_link::manage_symlinks,
    user_config::{get_xdg_user_config_path, make_config},
    MainApp,
};

// so we can use ?
fn run() -> Result<(), Box<dyn Error>> {
    let yml = load_yaml!("cli.yml");
    let arg_matches = App::from_yaml(yml).get_matches();

    // TODO: make it possible to run without config
    let config_path = if let Some(path) = arg_matches.value_of("config") {
        make_then_check_path(&[path]).ok_or("bad config from cli")?
    } else {
        get_xdg_user_config_path()?
    };

    // get run time options from config file
    let config = make_config(&config_path);
    let main = MainApp::new(arg_matches, config)?;

    main.verbose_ouput("got main app", None);

    // make sure the git repo is cloned local and up both the server and the
    // local copy are synced
    manage_git(&main)?;

    // make the needed symlinks
    manage_symlinks(&main)
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
    }
}
