use directories::ProjectDirs;

lazy_static! {
    pub static ref DIRS: Option<ProjectDirs> = ProjectDirs::from("me", "cosarara",  "fucking-weeb");
}

pub fn dirs() -> &'static directories::ProjectDirs {
    let dirs = DIRS.as_ref().unwrap();
    dirs
}
