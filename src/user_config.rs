use std::env;
use std::error::Error;
use std::path::PathBuf;

use toml::Value;

use crate::auto_filters::FilterType;
use crate::my_utils::{make_then_check_path, open_file_or_panic};

#[derive(Debug)]
pub struct Target {
    pub name: String,
    pub file_patterns: Vec<String>,
}

pub struct Conf {
    pub under_dir: String,
    pub upper_dir: String,
    pub filter_type: FilterType,
    pub to_filter: Vec<Target>,
    pub git_url: Option<String>,
}

impl Conf {
    fn new(toml_val: Value) -> Result<Self, Box<dyn Error>> {
        let base_vars: &Value = match toml_val.get("base_vars") {
            Some(base_vars) => base_vars,
            None => return Err(Box::from("need base vars")),
        };

        let base_dir = if let Some(b_str) = base_vars.get("base") {
            b_str.clone().to_string()
        } else {
            env::var("HOME").map_err(|err| format!("no HOME var {}", err))?
        };

        let upper_dir = match base_vars.get("upper_dir") {
            Some(upper_dir) => upper_dir.to_string(),
            None => base_dir.clone(),
        };

        let under_dir = match base_vars.get("under_dir") {
            Some(under_dir) => under_dir.clone().to_string(),
            None => return Err(Box::from("need under_dir")),
        };

        let git_url = match base_vars.get("git_url") {
            Some(git_url) => Some(git_url.clone().to_string()),
            None => None,
        };

        let filter_rules: Option<&Value> = toml_val.get("filter_rules");

        let filter_type = if let Some(f_rules) = filter_rules {
            match f_rules.get("filter_type") {
                Some(maybe_str) => match maybe_str.as_str() {
                    Some(val) if val == "hostname" => FilterType::Hostname,
                    _ => FilterType::Keyfile,
                },
                None => FilterType::Keyfile,
            }
        } else {
            FilterType::Keyfile
        };

        let mut to_filter: Vec<Target> = vec![];

        for (key, value) in toml_val.as_table().unwrap() {
            if let Some(target) = value.get("file_patterns") {
                let name = key.to_owned();
                let file_patterns: Vec<String> = target
                    .as_array()
                    .map(|vec| {
                        vec.iter().map(|ele| ele.as_str().unwrap().to_owned())
                    })
                    .ok_or("can't get array from patterns")?
                    .collect();

                to_filter.push(Target {
                    name,
                    file_patterns,
                });
            }
        }

        if to_filter.is_empty() {
            return Err(Box::from("no filters given"));
        }

        Ok(Conf {
            filter_type,
            upper_dir,
            under_dir,
            to_filter,
            git_url,
        })
    }
}

pub fn make_config(config_path: &PathBuf) -> Result<Conf, Box<dyn Error>> {
    let toml_string: String = open_file_or_panic(config_path);
    let toml: Value = toml_string.parse::<Value>()?;
    Conf::new(toml)
}

pub fn get_xdg_user_config_path() -> Result<PathBuf, Box<dyn Error>> {
    match env::var("XDG_CONFIG_HOME") {
        Ok(var) => make_then_check_path(&[&var, "manage"])
            .ok_or_else(|| Box::from("config dose not exist")),
        Err(err) => Err(Box::from(format!("no XDG_CONFIG_HOME {}", err))),
    }
}

#[cfg(test)]
mod test {
    // thanks https://medium.com/@ericdreichert/
    //       test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab
    use std::borrow::Borrow;
    use std::fs;
    use std::panic;
    use tempfile::tempdir;

    use super::*;

    struct TestData {
        dir_vec: Vec<PathBuf>,
        file_vec: Vec<PathBuf>,
        fake_path: PathBuf,
    }

    impl TestData {
        fn new(
            dir_vec: Vec<PathBuf>,
            file_vec: Vec<PathBuf>,
            fake_path: PathBuf,
        ) -> Self {
            TestData {
                dir_vec,
                file_vec,
                fake_path,
            }
        }
    }

    fn setup_temp<Y, T>(fake_data: Y, test: T)
    where
        Y: FnOnce(&PathBuf) -> TestData,
        T: FnOnce(&PathBuf, &PathBuf),
    {
        let test_dir = tempdir().unwrap();

        let temp_path = test_dir.into_path();

        let data: TestData = fake_data(&temp_path);

        let fake_path = data.fake_path;
        let files_to_make = data.file_vec;
        let dirs_to_make = data.dir_vec;

        for dir in dirs_to_make {
            if let Err(err) = fs::create_dir_all(&dir) {
                assert!(false, "dir {:?} make error {}", dir, err);
            }
        }

        for file in files_to_make {
            if let Err(err) = fs::File::create(&file) {
                assert!(false, "file {:?} make error {}", file, err);
            }
        }

        assert!(fake_path.exists(), "tempdir fails");

        test(&temp_path, &fake_path);

        assert!(true, "idk how this would fail");
    }

    fn fake_config_data(test_dir: &PathBuf) -> TestData {
        let dir_vec: Vec<PathBuf> = vec![".config/manage"]
            .iter()
            .map(|val| test_dir.join(val))
            .collect();

        let file_vec: Vec<PathBuf> = vec![".config/manage/config"]
            .iter()
            .map(|val| test_dir.join(val))
            .collect();

        let fake_path = test_dir.join(".config/manage/config");

        TestData::new(dir_vec, file_vec, fake_path)
    }

    #[test]
    fn test_make_config() {
        setup_temp(fake_config_data, |_, fake_config_path| {
            let fake_config = r#"
                [base_vars]

                # no default
                under_dir = '.dots'

                [filter_rules]
                filter_type = 'hostname'

                [Monolith]
                file_patterns = ['*_all', '*_M']

                [Odimm]
                file_patterns = ['*_all', '*_O']
                "#;

            if let Err(err) = fs::write(fake_config_path, fake_config) {
                assert!(false, "{}", err);
            }

            if let Ok(conf) = make_config(fake_config_path) {
                assert!(true, "made config");
                assert!(!conf.under_dir.is_empty(), "didn't get under dir");
                assert!(!conf.to_filter.is_empty(), "didn't get to_filter");
            } else {
                assert!(false, "broken config");
                panic!("fuck");
            }
        })
    }

    #[test]
    fn test_make_broken_config_under() {
        setup_temp(fake_config_data, |_, fake_config_path| {
            let fake_config = r#"
                [base_vars]

                # no default
                # under_dir = '.dots'

                [filter_rules]
                filter_type = 'hostname'

                [Test_Comp]
                file_patterns = ['*_all', '*_O']
                "#;

            if let Err(err) = fs::write(fake_config_path, fake_config) {
                assert!(false, "{}", err);
            }

            if let Err(err) = make_config(fake_config_path) {
                let error: &Error = err.borrow();
                if format!("{}", error) == "need under_dir" {
                    assert!(true, "{}", err);
                } else {
                    assert!(false, "{}", err);
                }
            } else {
                assert!(false, "made config");
            }
        })
    }

    #[test]
    fn test_make_broken_config_filter_type() {
        setup_temp(fake_config_data, |_, fake_config_path| {
            let fake_config = r#"
                [base_vars]

                # no default
                under_dir = '.dots'

                # [filter_rules]
                # filter_type = 'hostname'

                # [Test_Comp]
                # file_patterns = ['*_all', '*_O']
                "#;

            if let Err(err) = fs::write(fake_config_path, fake_config) {
                assert!(false, "{}", err);
            }

            if let Err(err) = make_config(fake_config_path) {
                let error: &Error = err.borrow();
                if format!("{}", error) == "no filters given" {
                    assert!(true, "{}", err);
                } else {
                    assert!(false, "{}", err);
                }
            } else {
                assert!(false, "made config");
            }
        })
    }
}
