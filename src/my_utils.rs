use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

// make a PathBuf joining the strs to the end
fn str_to_path(to_add_collection: &[&str]) -> PathBuf {
    let mut path_accu: PathBuf = PathBuf::new();

    for to_add in to_add_collection {
        // TODO: find out why . in toml string fucks up the whole str
        path_accu = path_accu.join(to_add.replace("\"", ""));
    }

    path_accu
}

// run a sanity check on a path
fn check_path(path: PathBuf) -> Option<PathBuf> {
    if path.exists() {
        Some(path.canonicalize().expect("cant canonicalize path"))
    } else {
        None
    }
}

pub fn make_then_check_path(strs_to_add: &[&str]) -> Option<PathBuf> {
    let maybe_path = str_to_path(strs_to_add);

    check_path(maybe_path)
}

pub fn open_file_or_panic(path: &PathBuf) -> String {
    match File::open(path) {
        Ok(mut file) => {
            let mut buffer_string = String::new();
            file.read_to_string(&mut buffer_string)
                .expect("cant get file to string");

            buffer_string
        }
        Err(err) => panic!("{}", err),
    }
}

// TODO: this is trash and i feel bad
pub fn get_linux_hostname() -> Option<String> {
    let h_out = Command::new("hostname")
        .output()
        .expect("failed to execute process");

    let to_ret = String::from_utf8_lossy(&h_out.stdout).trim().to_owned();

    Some(to_ret)
}

pub fn vec_to_string<T>(message: &str, vec_to_join: &[T]) -> String
where
    T: fmt::Display,
{
    vec_to_join
        .iter()
        .map(|sym| format!("{} {}", message, sym))
        .collect::<Vec<String>>()
        .join("\n")
}
