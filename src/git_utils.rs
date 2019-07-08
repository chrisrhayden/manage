use git2::{self, Repository};
use std::error::Error;
use std::io;

use crate::MainApp;

fn initial_clone(main_app: &MainApp) -> Result<Repository, Box<dyn Error>> {
    let git_url: String = if main_app.git_url.is_none() {
        return Err(Box::from("no git url"));
    } else {
        main_app.git_url.clone().unwrap()
    };

    println!("would you like to clone the given url");
    println!("{}", git_url);

    let mut user_out = String::new();
    io::stdin()
        .read_line(&mut user_out)
        .expect("couldn't get line");

    if user_out.trim().to_lowercase() == "y" {
        Repository::clone(&git_url, &main_app.under_dir).map_err(Box::from)
    } else {
        Err(Box::from("user canceled"))
    }
}

pub fn manage_git(main_app: &MainApp) -> Result<(), Box<dyn Error>> {
    let repo = match Repository::open(&main_app.under_dir) {
        Err(err) => {
            eprintln!("{}", err);
            initial_clone(main_app)?
        }
        Ok(repo) => repo,
    };

    // more git stuff here

    Ok(())
}
