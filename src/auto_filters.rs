use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::fs::ReadDir;
use std::path::PathBuf;

use crate::my_utils::get_linux_hostname;
use crate::user_config::{Conf, Target};

pub type FoundResult = Result<Vec<PathBuf>, Box<dyn Error>>;

#[derive(Debug)]
pub enum FilterType {
    Keyfile,
    Hostname,
}

fn pattern_pred(val: &OsStr, pat: &str) -> Result<bool, Box<dyn Error>> {
    if pat.starts_with('*') {
        Ok(val.to_str().unwrap().ends_with(&pat.replace("*", "")))
    } else if pat.ends_with('*') {
        Ok(val.to_str().unwrap().starts_with(&pat.replace("*", "")))
    } else {
        Err(Box::from("bad filter"))
    }
}

fn get_dirs(read_dir: ReadDir, file_patterns: &[String]) -> FoundResult {
    let mut to_ret: Vec<PathBuf> = vec![];

    for dir in read_dir {
        let dir = dir?;
        let dir_path = dir.path();
        let file_name = dir_path.file_name().ok_or("cant get file name")?;

        for pat in file_patterns {
            if pattern_pred(&file_name, &pat)? {
                to_ret.push(dir_path.clone());
            }
        }
    }

    if to_ret.is_empty() {
        Err(Box::from("didn't find any under dirs"))
    } else {
        Ok(to_ret)
    }
}

fn hostname_filter(under_dir: &PathBuf, to_filter: &[Target]) -> FoundResult {
    let hostname = get_linux_hostname().ok_or("cant get host name")?;

    let target: Vec<&Target> = to_filter
        .iter()
        .filter(|targ| targ.name == hostname)
        .collect();

    if target.len() != 1 {
        return Err(Box::from("more then one filter found for host in config"));
    }

    let target: &Target = target.first().expect("cant get filter");

    let read_under = fs::read_dir(under_dir)?;

    get_dirs(read_under, &target.file_patterns)
}

pub fn filter_target_dirs(under_dir: &PathBuf, conf: &Conf) -> FoundResult {
    match conf.filter_type {
        FilterType::Keyfile => unimplemented!(),
        FilterType::Hostname => hostname_filter(under_dir, &conf.to_filter),
    }
}

#[cfg(test)]
mod test {
    use tempfile::tempdir;

    use super::*;

    struct TestData {
        dir_vec: Vec<PathBuf>,
        file_vec: Vec<PathBuf>,
        check_path: PathBuf,
    }

    impl TestData {
        fn new(
            dir_vec: Vec<PathBuf>,
            file_vec: Vec<PathBuf>,
            check_path: PathBuf,
        ) -> Self {
            TestData {
                dir_vec,
                file_vec,
                check_path,
            }
        }
    }

    // TODO: find edge cases
    fn setup_temp<Y, T>(fake_data: Y, test: T)
    where
        Y: FnOnce(&PathBuf) -> TestData,
        T: FnOnce(&PathBuf, &PathBuf),
    {
        let test_dir = tempdir().unwrap();

        let temp_path = test_dir.into_path();

        let data: TestData = fake_data(&temp_path);

        let check_path = data.check_path;
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

        assert!(check_path.exists(), "tempdir fails");

        test(&temp_path, &check_path);

        assert!(true, "idk how this would fail");
    }

    fn fake_under_data_start(system_dir: &PathBuf) -> TestData {
        let dirs_to_make: Vec<PathBuf> = vec![
            "under_dir/all_fake_zsh",
            "under_dir/all_fake_i3",
            "under_dir/O_fake_polybar",
            "under_dir/M_fake_polybar",
            "under_dir/O_fake_termite",
            "under_dir/M_fake_termite",
        ]
        .iter()
        .map(|dir| system_dir.join(dir))
        .collect();

        let check_path = system_dir.join("under_dir");

        TestData::new(dirs_to_make, vec![], check_path)
    }

    fn hand_made_data_start(system_dir: &PathBuf) -> Vec<PathBuf> {
        vec![
            "under_dir/all_fake_zsh",
            "under_dir/all_fake_i3",
            "under_dir/O_fake_polybar",
            "under_dir/O_fake_termite",
        ]
        .iter()
        .map(|dir| system_dir.join(dir))
        .collect()
    }

    fn fake_under_data_end(system_dir: &PathBuf) -> TestData {
        let dirs_to_make: Vec<PathBuf> = vec![
            "under_dir/fake_zsh_all",
            "under_dir/fake_i3_all",
            "under_dir/fake_polybar_M",
            "under_dir/fake_polybar_O",
            "under_dir/fake_termite_M",
            "under_dir/fake_termite_O",
        ]
        .iter()
        .map(|dir| system_dir.join(dir))
        .collect();

        let check_path = system_dir.join("under_dir");

        TestData::new(dirs_to_make, vec![], check_path)
    }

    fn hand_made_data_end(system_dir: &PathBuf) -> Vec<PathBuf> {
        vec![
            "under_dir/fake_zsh_all",
            "under_dir/fake_i3_all",
            "under_dir/fake_polybar_M",
            "under_dir/fake_termite_M",
        ]
        .iter()
        .map(|dir| system_dir.join(dir))
        .collect()
    }

    #[test]
    fn test_get_dirs_end_pat() {
        setup_temp(fake_under_data_end, |temp_path, under_dir| {
            let read_under = match fs::read_dir(under_dir) {
                Ok(val) => val,
                Err(err) => panic!("{}", err),
            };

            let hand_made_data = hand_made_data_end(&temp_path);

            if let Ok(found_vec) =
                get_dirs(read_under, &["*_all".to_string(), "*_M".to_string()])
            {
                for found in found_vec {
                    assert!(
                        hand_made_data.contains(&found),
                        "didn't find the right dirs"
                    )
                }
            } else {
                assert!(false, "get_dirs failed");
            }
        })
    }

    #[test]
    fn test_get_dirs_start_pat() {
        setup_temp(fake_under_data_start, |temp_path, under_dir| {
            let read_under = match fs::read_dir(under_dir) {
                Ok(val) => val,
                Err(err) => panic!("{}", err),
            };

            let hand_made_data = hand_made_data_start(&temp_path);

            if let Ok(found_vec) =
                get_dirs(read_under, &["all_*".to_string(), "O_*".to_string()])
            {
                for found in found_vec {
                    assert!(
                        hand_made_data.contains(&found),
                        "didn't find the right dirs"
                    )
                }
            } else {
                assert!(false, "get_dirs failed");
            }
        })
    }
}
