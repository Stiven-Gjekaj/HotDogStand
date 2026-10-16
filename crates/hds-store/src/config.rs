//! The settings of the application, which are the same for each workspace:
//! the theme, the frame, and the workspaces that the person opened last.
//!
//! They live in a small text file in the data directory, with one
//! `key = value` on each line, so a person can read it and correct it.

use std::path::{Path, PathBuf};

use crate::{Error, Result, WORKSPACE_VAR, data_dir};

/// The variable that gives a different path for the settings file.
pub const CONFIG_VAR: &str = "HOTDOGSTAND_CONFIG";

/// The number of workspaces that the list of recent workspaces keeps.
pub const RECENT_MAX: usize = 8;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// `None` for a setting that the file does not give yet.
    pub theme: Option<String>,
    pub dark: Option<bool>,
    pub system_frame: Option<bool>,
    /// The workspace files, the last one opened first.
    pub recent: Vec<PathBuf>,
}

/// Gives the path of the settings file: the value of [`CONFIG_VAR`] when it
/// is set. When only [`WORKSPACE_VAR`] is set, as in a test, the file goes
/// beside that workspace, so a test never writes the settings of the person.
pub fn config_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(CONFIG_VAR).filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    if let Some(workspace) = std::env::var_os(WORKSPACE_VAR).filter(|p| !p.is_empty()) {
        let workspace = PathBuf::from(workspace);
        let dir = workspace.parent().unwrap_or(Path::new("."));
        return Ok(dir.join("hotdogstand-settings.txt"));
    }
    Ok(data_dir()?.join("settings.txt"))
}

impl Config {
    /// Reads the settings file. A file that is not there gives the defaults.
    pub fn load(path: &Path) -> Result<Config> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Config::parse(&text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(error) => Err(Error::Io(error)),
        }
    }

    /// Writes the settings file. It writes a new file and then puts it in the
    /// place of the old one, so a crash leaves the old file or the new one.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let next = path.with_extension("txt.new");
        std::fs::write(&next, self.to_text())?;
        std::fs::rename(&next, path)?;
        Ok(())
    }

    /// Reads the text of a settings file. A line that it does not know, such
    /// as a setting of a newer version, it passes over.
    pub fn parse(text: &str) -> Config {
        let mut config = Config::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "theme" => config.theme = Some(value.to_owned()),
                "dark" => config.dark = parse_bool(value),
                "system_frame" => config.system_frame = parse_bool(value),
                "recent" if !value.is_empty() && config.recent.len() < RECENT_MAX => {
                    config.recent.push(PathBuf::from(value));
                }
                _ => {}
            }
        }
        config
    }

    pub fn to_text(&self) -> String {
        let mut text =
            String::from("# The settings of HotDogStand. The application writes this file.\n");
        if let Some(theme) = &self.theme {
            text.push_str(&format!("theme = {theme}\n"));
        }
        if let Some(dark) = self.dark {
            text.push_str(&format!("dark = {dark}\n"));
        }
        if let Some(frame) = self.system_frame {
            text.push_str(&format!("system_frame = {frame}\n"));
        }
        for path in &self.recent {
            text.push_str(&format!("recent = {}\n", path.display()));
        }
        text
    }

    /// Puts a workspace at the top of the list of recent workspaces.
    pub fn opened(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_owned());
        self.recent.truncate(RECENT_MAX);
    }

    /// Takes a workspace off the list, for a file that is not there now.
    pub fn forget(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_comes_back_the_same() {
        let config = Config {
            theme: Some("hot-dog-stand".into()),
            dark: Some(true),
            system_frame: Some(false),
            recent: vec![
                "/home/ana/Office.hotdogstand".into(),
                "C:\\Data\\Home.db".into(),
            ],
        };
        assert_eq!(Config::parse(&config.to_text()), config);
    }

    #[test]
    fn an_empty_text_gives_no_settings() {
        assert_eq!(Config::parse(""), Config::default());
    }

    #[test]
    fn unknown_lines_and_bad_values_are_passed_over() {
        let config = Config::parse(
            "# a comment\nfuture_setting = 3\ndark = maybe\nno equals sign\n  theme  =  aero  \n",
        );
        assert_eq!(config.theme.as_deref(), Some("aero"));
        assert_eq!(config.dark, None);
        assert!(config.recent.is_empty());
    }

    #[test]
    fn a_path_with_an_equals_sign_stays_whole() {
        let config = Config::parse("recent = /home/ana/a=b.hotdogstand\n");
        assert_eq!(config.recent, [PathBuf::from("/home/ana/a=b.hotdogstand")]);
    }

    #[test]
    fn opened_puts_a_workspace_first_and_once() {
        let mut config = Config::default();
        for name in ["a", "b", "c", "b"] {
            config.opened(Path::new(name));
        }
        assert_eq!(config.recent, ["b", "c", "a"].map(PathBuf::from));
        config.forget(Path::new("c"));
        assert_eq!(config.recent, ["b", "a"].map(PathBuf::from));
    }

    #[test]
    fn the_list_keeps_the_last_eight() {
        let mut config = Config::default();
        for i in 0..12 {
            config.opened(Path::new(&format!("w{i}")));
        }
        assert_eq!(config.recent.len(), RECENT_MAX);
        assert_eq!(config.recent[0], PathBuf::from("w11"));
        assert_eq!(config.recent[7], PathBuf::from("w4"));
    }

    #[test]
    fn save_and_load_use_the_file() {
        let dir = std::env::temp_dir().join(format!("hds-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("settings.txt");

        assert_eq!(Config::load(&path).unwrap(), Config::default());
        let mut config = Config {
            dark: Some(true),
            ..Config::default()
        };
        config.opened(Path::new("/tmp/one.hotdogstand"));
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), config);
        assert!(!path.with_extension("txt.new").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
