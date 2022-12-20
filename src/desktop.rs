use freedesktop_entry_parser::parse_entry;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::{
    collections::{BTreeMap, HashSet},
    env,
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

//

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DesktopEntry {
    name: String,
    search: String,
    exec: String,
    term: bool,
    path: String,
}

pub struct DesktopEntries {
    entries: HashSet<DesktopEntry>,
    matcher: SkimMatcherV2,
}

//

impl DesktopEntries {
    pub fn new() -> Self {
        let data_home = env::var("XDG_DATA_HOME").ok();
        let data_dirs = env::var("XDG_DATA_DIRS").ok();

        let entries = [data_home, data_dirs]
            .iter()
            .flatten()
            .flat_map(|s| s.split(':'))
            .rev()
            .flat_map(|dir| std::fs::read_dir(Path::new(dir).join("applications")).ok())
            .flatten()
            .flatten()
            .filter_map(|s| {
                let ty = s.file_type().ok()?;
                ty.is_file().then_some(s.path())
            })
            .filter(|s| {
                s.extension()
                    .and_then(OsStr::to_str)
                    .map(str::to_lowercase)
                    .map_or(false, |s| &s == "desktop")
            })
            .filter_map(|desktop_file| {
                let path = desktop_file.as_os_str().to_string_lossy().to_string();

                let entry = parse_entry(desktop_file).ok()?;
                let desktop = entry.section("Desktop Entry");

                let exec = desktop.attr("Exec")?.to_string();
                let name = desktop.attr("Name")?.to_string();
                let comment = desktop.attr("Comment");
                let categories = desktop.attr("Categories");
                let keywords = desktop.attr("Keywords");
                let term = desktop.attr("Terminal") == Some("true");

                let search = [
                    Some(name.as_str()),
                    Some("\n"),
                    comment,
                    Some("\n"),
                    categories,
                    Some("\n"),
                    keywords,
                ]
                .into_iter()
                .flatten()
                .collect::<String>();

                Some(DesktopEntry {
                    name,
                    search,
                    exec,
                    term,
                    path,
                })
            })
            .collect::<HashSet<_>>();

        let matcher = SkimMatcherV2::default();

        Self { entries, matcher }
    }

    pub fn matches(&self, pattern: &str) -> BTreeMap<i64, &DesktopEntry> {
        self.entries
            .iter()
            .filter_map(|s| Some((self.matcher.fuzzy_match(&s.search, pattern)?, s)))
            .collect::<BTreeMap<_, _>>()
    }
}

impl DesktopEntry {
    pub fn launch(&self) {
        let exec = self
            .exec
            .replace("%f", "") // replace with a single file
            .replace("%F", "") // replace with a list of files
            .replace("%u", "") // replace with a single url
            .replace("%U", "") // replace with a list of urls
            .replace("%d", "") // deprecated
            .replace("%D", "") // deprecated
            .replace("%n", "") // deprecated
            .replace("%N", "") // deprecated
            .replace("%i", "") // replace with '--icon <Icon>' and replace '<Icon>' with the Icon key value
            .replace("%c", self.name.as_str()) // replace with Name key value
            .replace("%k", self.path.as_str()) // replace with the corresponding full `.desktop` file path
            .replace("%v", "") // deprecated
            .replace("%m", ""); // deprecated
        let mut args = exec.split_whitespace();
        let cmd = args.next().expect("Exec has nothing in it");
        tracing::info!("Launched {cmd}");

        if cmd.contains("://") {
            Self::launch_url(cmd, args)
        } else {
            Self::launch_app(cmd, args)
        }
    }

    fn launch_app<'a>(cmd: &'a str, args: impl Iterator<Item = &'a str>) {
        Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to launch");
    }

    fn launch_url<'a>(cmd: &'a str, args: impl Iterator<Item = &'a str>) {
        let path: OsString = [cmd]
            .into_iter()
            .chain(args)
            .flat_map(|s| [s, "\n"])
            .collect::<String>()
            .into();
        let path = path.as_os_str();
        let open_handlers = [
            ("xdg-open", &[path] as &[_]),
            ("gio", &[OsStr::new("open"), path]),
            ("gnome-open", &[path]),
            ("kde-open", &[path]),
        ];

        for (command, args) in &open_handlers {
            let result = Command::new(command)
                .args(*args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .and_then(|mut process| process.wait());

            if result.map_or(false, |s| s.success()) {
                return;
            }
        }
    }
}
