use std::env::set_current_dir;
use std::error::Error;
use std::fmt;
use std::fs;
use std::os::unix;
use std::path::PathBuf;

use crate::{my_utils::vec_to_string, Action, MainApp};

type WorkResult = Result<bool, Box<dyn Error>>;

#[derive(Debug)]
pub struct SymLink {
    pub upper_file: PathBuf,
    pub target_file: PathBuf,
    pub exists: bool,
}

impl SymLink {
    pub fn new(up: &PathBuf, lo: &PathBuf, exists: bool) -> Self {
        SymLink {
            upper_file: up.to_owned(),
            target_file: lo.to_owned(),
            exists,
        }
    }

    fn delete_symlink(&self) -> WorkResult {
        if !self.exists {
            return Ok(false);
        }

        if self.upper_file == self.target_file {
            return Err(Box::from("upper_dir is pointing to target_dir"));
        }

        match fs::remove_file(&self.upper_file) {
            Err(err) => Err(Box::from(format!("cant delete symlink {}", err))),
            Ok(_) => Ok(true),
        }
    }

    fn make_symlink(&self) -> WorkResult {
        if self.exists {
            return Ok(false);
        }

        if let Err(err) = unix::fs::symlink(&self.target_file, &self.upper_file)
        {
            Err(Box::from(format!("cant make symlink {}", err)))
        } else {
            Ok(true)
        }
    }
}

impl fmt::Display for SymLink {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let target = self.target_file.to_str().expect("cant display target");;
        let upper = self.upper_file.to_str().expect("cant display upper");
        write!(f, "{} -> {}", upper, target)
    }
}

fn dry_runner(sym: &SymLink, main_app: &MainApp) -> WorkResult {
    let work = match main_app.action {
        Action::Remake => {
            println!("would remake {}", sym);
            true
        }
        Action::Make => {
            if sym.exists {
                println!("already made {}", sym);
                false
            } else {
                println!("would make {}", sym);
                true
            }
        }
        Action::Delete => {
            if sym.exists {
                println!("would delete {}", sym);
                true
            } else {
                println!("already missing {}", sym);
                false
            }
        }
    };

    Ok(work)
}

fn live_runner(sym: &SymLink, main_app: &MainApp) -> WorkResult {
    let work = match main_app.action {
        Action::Make => sym.make_symlink()?,
        Action::Delete => sym.delete_symlink()?,
        _ => return Err(Box::from("bad action")),
    };

    Ok(work)
}

pub fn run_sym_vec(to_sym_vec: &[SymLink], main_app: &MainApp) -> WorkResult {
    if to_sym_vec.is_empty() {
        return Ok(false);
    }

    let mut did_work = false;
    for sym in to_sym_vec {
        let work = if main_app.dry_run {
            dry_runner(&sym, main_app)?
        } else {
            live_runner(&sym, main_app)?
        };

        if work {
            did_work = true;
        }
    }

    let msg = if did_work {
        format!("{}", main_app.action)
    } else {
        "did noting".to_string()
    };

    main_app.verbose_ouput("", Some(&vec_to_string(&msg, &to_sym_vec)));

    Ok(did_work)
}

// return symlinks we own return err on symlinks we dont
pub fn symlink_check(
    real_maybe: &PathBuf,
    maybe_path: &PathBuf,
    target_path: &PathBuf,
    upper_dir: &PathBuf,
) -> Result<SymLink, Box<dyn Error>> {
    // idk why but real_maybe was being made relative
    // ../.under/thing/other
    let real_maybe = upper_dir
        .join(real_maybe)
        .canonicalize()
        .map_err(|err| format!("cant canonicalize real_maybe {}", err))?;

    if target_path.as_os_str() == real_maybe.as_os_str() {
        Ok(SymLink::new(&maybe_path, &target_path, true))
    } else {
        Err(Box::from(format!(
            "link is not owned by us {:?}",
            maybe_path
        )))
    }
}

pub fn get_symlink_vec(
    upper_dir: &PathBuf,
    target_dir: &PathBuf,
) -> Result<Vec<SymLink>, Box<dyn Error>> {
    let read_target_dir =
        fs::read_dir(target_dir).expect("cant read target_path");

    let mut to_ret: Vec<SymLink> = vec![];

    for dir in read_target_dir {
        let dir = dir.expect("didn't get dir?");
        let target_path = dir.path();
        let target_file_name = dir.file_name();

        let maybe_path = upper_dir.join(&target_file_name);

        if let Ok(real_maybe) = maybe_path.read_link() {
            let sym = symlink_check(
                &real_maybe,
                &maybe_path,
                &target_dir,
                &upper_dir,
            )?;

            to_ret.push(sym);
        } else if maybe_path.is_dir() {
            match get_symlink_vec(&maybe_path, &target_path) {
                Ok(mut new_to_ret) => to_ret.append(&mut new_to_ret),
                Err(err) => return Err(err),
            };
        } else if !maybe_path.exists() {
            to_ret.push(SymLink::new(&maybe_path, &target_path, false));
        } else {
            return Err(Box::from(format!("file exists {:?}", maybe_path)));
        }
    }

    Ok(to_ret)
}

pub fn manage_symlinks(main_app: &MainApp) -> Result<(), Box<dyn Error>> {
    set_current_dir("/").expect("cant change dir");

    let target_dirs: &Vec<PathBuf> = &main_app.target_dirs;
    let upper_dir: &PathBuf = &main_app.upper_dir;

    let map_low_to_sym_vec = target_dirs
        .iter()
        .map(|l_d| get_symlink_vec(upper_dir, l_d));

    let mut did_work = false;
    for sym_vec in map_low_to_sym_vec {
        match sym_vec {
            // this is probably unnecessary
            Ok(ref syms) => {
                if run_sym_vec(syms, main_app)? {
                    did_work = true;
                }
            }
            Err(err) => eprintln!("Symlink Error {}", err),
        }
    }

    if did_work {
        main_app.verbose_ouput(&format!("{} link[s]", main_app.action), None);
    } else {
        main_app.verbose_ouput("nothing to do", None);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    // thanks https://medium.com/@ericdreichert/
    //       test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab
    use std::fs;
    use std::os::unix;
    use std::panic;
    use std::path::PathBuf;

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

    // TODO: find edge cases
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

    fn fake_zsh_data(system_dir: &PathBuf) -> TestData {
        let dirs_to_make: Vec<PathBuf> =
            vec!["zshrc.d", ".under/fake_zsh/zshrc.d"]
                .iter()
                .map(|dir| system_dir.join(dir))
                .collect();

        let files_to_make: Vec<PathBuf> = vec![
            ".under/fake_zsh/zshrc",
            ".under/fake_zsh/zshrc.d/zshrc_conf",
        ]
        .iter()
        .map(|file| system_dir.join(file))
        .collect();

        let fake_path = system_dir.join(".under/fake_zsh");

        TestData::new(dirs_to_make, files_to_make, fake_path)
    }

    fn hand_made_zsh_data(system_path: &PathBuf) -> Vec<PathBuf> {
        vec!["zshrc.d/zshrc_conf", "zshrc"]
            .iter()
            .map(|file| system_path.join(file))
            .collect()
    }

    fn fake_i3_data(system_dir: &PathBuf) -> TestData {
        let dirs_to_make: Vec<PathBuf> =
            vec![".config", ".under/fake_i3/.config/i3"]
                .iter()
                .map(|dir| system_dir.join(dir))
                .collect();

        let files_to_make: Vec<PathBuf> =
            vec![".under/fake_i3/.config/i3/i3_conf"]
                .iter()
                .map(|file| system_dir.join(file))
                .collect();

        let fake_path = system_dir.join(".under/fake_i3");

        TestData::new(dirs_to_make, files_to_make, fake_path)
    }

    fn hand_made_i3_data(temp_path: &PathBuf) -> Vec<PathBuf> {
        vec![".config/i3"]
            .iter()
            .map(|file| temp_path.join(file))
            .collect()
    }

    fn fake_main(tmp: &PathBuf, action: Action) -> MainApp {
        MainApp::new(Some(tmp.to_owned()), None, action, false, 0, None, None)
            .unwrap()
    }

    #[test]
    fn test_get_symlink_paths_fake_zsh() {
        setup_temp(fake_zsh_data, |temp_path, fake_under_zsh| {
            let hand_test_zsh: Vec<PathBuf> = hand_made_zsh_data(&temp_path);

            let to_sym = match get_symlink_vec(temp_path, fake_under_zsh) {
                Ok(sym) => sym,
                Err(err) => {
                    assert!(false, "failed to get_symlink_paths {}", err);
                    return; // lol
                }
            };

            let flat_syms: Vec<PathBuf> = to_sym
                .iter()
                .flat_map(|sym| {
                    vec![sym.upper_file.to_owned(), sym.target_file.to_owned()]
                })
                .collect();

            for hand in hand_test_zsh {
                assert!(
                    flat_syms.contains(&hand),
                    "hand made sym {:?} not found",
                    hand
                );
            }
        })
    }

    #[test]
    fn test_get_symlink_paths_fake_i3() {
        setup_temp(fake_i3_data, |temp_path, fake_under_i3| {
            let hand_test_links: Vec<PathBuf> = hand_made_i3_data(temp_path);

            let to_sym = match get_symlink_vec(temp_path, fake_under_i3) {
                Ok(sym) => sym,
                Err(err) => {
                    assert!(false, "failed to get_symlink_paths shit {}", err);
                    return; // lol
                }
            };

            let flat_syms: Vec<PathBuf> = to_sym
                .iter()
                .flat_map(|sym| {
                    vec![sym.upper_file.to_owned(), sym.target_file.to_owned()]
                })
                .collect();

            for hand in &hand_test_links {
                assert!(
                    flat_syms.contains(&hand),
                    "hand made sym {:?} not found",
                    hand
                );
            }
        })
    }

    #[test]
    fn test_make_sym_zsh() {
        setup_temp(fake_zsh_data, |temp_path, _| {
            let syms = vec![
                SymLink::new(
                    &temp_path.join("zshrc"),
                    &temp_path.join(".under/fake_zsh/zshrc"),
                    false,
                ),
                SymLink::new(
                    &temp_path.join("zshrc.d/zshrc_conf"),
                    &temp_path.join(".under/fake_zsh/zshrc.d/zshrc_conf"),
                    false,
                ),
            ];

            let main = fake_main(temp_path, Action::Make);

            if let Err(err) = run_sym_vec(&syms, &main) {
                println!("{}", err);
                assert!(false, "cant make symlink")
            };

            for sym in syms {
                assert!(sym.upper_file.exists(), "didn't make symlink");

                let up_conz = sym.upper_file.canonicalize().unwrap();
                let up_str = up_conz.as_os_str();

                let lo_str = sym.target_file.as_os_str();

                assert!(up_str == lo_str, "not pointing to right place");
            }

            assert!(true);
        })
    }

    #[test]
    fn test_make_sym_i3() {
        setup_temp(fake_i3_data, |temp_path, _| {
            let syms = vec![SymLink::new(
                &temp_path.join(".config/i3"),
                &temp_path.join(".under/fake_i3/.config/i3"),
                false,
            )];

            let main = fake_main(temp_path, Action::Make);

            if let Err(err) = run_sym_vec(&syms, &main) {
                println!("{}", err);
                assert!(false, "cant make symlink")
            };

            for sym in syms {
                assert!(sym.upper_file.exists(), "didnt make symlink");

                let up_conz = sym.upper_file.canonicalize().unwrap();
                let up_str = up_conz.as_os_str();

                let lo_str = sym.target_file.as_os_str();

                assert!(up_str == lo_str, "not pointing to right place");
            }

            assert!(true);
        })
    }

    #[test]
    fn test_make_delete_symlink_zsh() {
        setup_temp(fake_zsh_data, |temp_path, _| {
            let syms = vec![
                SymLink::new(
                    &temp_path.join("zshrc.d/zshrc_conf"),
                    &temp_path.join(".under/fake_zsh/zshrc.d/zshrc_conf"),
                    true,
                ),
                SymLink::new(
                    &temp_path.join("zshrc"),
                    &temp_path.join(".under/fake_zsh/zshrc"),
                    true,
                ),
            ];

            let main = fake_main(temp_path, Action::Delete);

            for sym in &syms {
                assert!(!sym.upper_file.exists(), "file exists?");

                unix::fs::symlink(&sym.target_file, &sym.upper_file).unwrap();

                assert!(sym.upper_file.exists(), "didn't make sym");
            }

            if let Err(err) = run_sym_vec(&syms, &main) {
                println!("{}", err);
                assert!(false, "cant delete symlink")
            };

            for sym in syms {
                assert!(!sym.upper_file.exists(), "didn't remove symlink");
            }

            assert!(true);
        })
    }
}
