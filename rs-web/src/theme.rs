use crate::date::today;
use clap::ValueEnum;

/// The winter look Jagex gave the site from 7 December 2004 ("Winter
/// Website") until 5 January 2005: snow on the logo and the title screen's
/// boxes, snowflakes behind it and holly on every page's frame.
const CHRISTMAS: &str = "/img/themes/christmas/2004";
/// (month, day) it starts and the last it's up, as in 2004.
const CHRISTMAS_FROM: (u32, u32) = (12, 7);
const CHRISTMAS_UNTIL: (u32, u32) = (1, 5);

/// Each picture the winter look swaps, under `/img/`, and its winter one.
/// The rest of the frame's winter pictures (`wedge_c`, `wedge_g2`,
/// `wedge_h2`, `wbackground2`) are lost, so those stay as they are.
const CHRISTMAS_SWAPS: &[(&str, &str)] = &[
    ("edge_a.jpg", "edge_a.jpg"),
    ("edge_d.jpg", "edge_d.jpg"),
    ("background.jpg", "snowflakes.png"),
    ("title/rslogo.gif", "wrslogo.gif"),
];

/// Which look the site has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Theme {
    /// Christmas while it was up in 2004, plain the rest of the year.
    #[default]
    Auto,
    Christmas,
    Plain,
}

impl Theme {
    /// Whether the winter look is up now.
    pub fn christmas(self) -> bool {
        match self {
            Self::Auto => in_christmas(today()),
            Self::Christmas => true,
            Self::Plain => false,
        }
    }

    /// The URL of `path` (under `/img/`): its winter picture while that's
    /// up and it has one, else itself.
    pub fn img(self, path: &str) -> String {
        let swap = CHRISTMAS_SWAPS.iter().find(|(from, _)| *from == path);
        match swap {
            Some((_, to)) if self.christmas() => format!("{CHRISTMAS}/{to}"),
            _ => format!("/img/{path}"),
        }
    }
}

/// Whether `day` falls in the winter look's weeks, which run over the new
/// year.
fn in_christmas((_, month, day): (i32, u32, u32)) -> bool {
    (month, day) >= CHRISTMAS_FROM || (month, day) <= CHRISTMAS_UNTIL
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn christmas_runs_over_the_new_year() {
        assert!(!in_christmas((2026, 12, 6)));
        assert!(in_christmas((2026, 12, 7)));
        assert!(in_christmas((2026, 12, 31)));
        assert!(in_christmas((2027, 1, 1)));
        assert!(in_christmas((2027, 1, 5)));
        assert!(!in_christmas((2027, 1, 6)));
        assert!(!in_christmas((2027, 7, 1)));
    }

    #[test]
    fn pictures_swap_only_for_christmas() {
        assert_eq!(Theme::Plain.img("edge_a.jpg"), "/img/edge_a.jpg");
        assert_eq!(
            Theme::Christmas.img("edge_a.jpg"),
            "/img/themes/christmas/2004/edge_a.jpg"
        );
        assert_eq!(
            Theme::Christmas.img("title/rslogo.gif"),
            "/img/themes/christmas/2004/wrslogo.gif"
        );
        assert_eq!(Theme::Christmas.img("edge_c.jpg"), "/img/edge_c.jpg");
    }

    #[test]
    fn every_picture_exists() {
        let public = Path::new(env!("CARGO_MANIFEST_DIR")).join("public");
        for (from, to) in CHRISTMAS_SWAPS {
            for url in [format!("/img/{from}"), format!("{CHRISTMAS}/{to}")] {
                assert!(public.join(&url[1..]).is_file(), "{url}");
            }
        }
    }

    /// A page that shows a swapped picture must ask for it through `img()`,
    /// or it stays plain at Christmas.
    #[test]
    fn pages_ask_for_swapped_pictures() {
        fn walk(dir: &Path, found: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, found);
                    continue;
                }
                let text = fs::read_to_string(&path).unwrap_or_default();
                for (from, _) in CHRISTMAS_SWAPS {
                    if text.contains(&format!("src=/img/{from}")) {
                        found.push(format!("{}: {from}", path.display()));
                    }
                }
            }
        }
        let mut found = Vec::new();
        walk(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates"),
            &mut found,
        );
        assert!(found.is_empty(), "{found:#?}");
    }
}
