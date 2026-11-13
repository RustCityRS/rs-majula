use crate::active_player::ActivePlayer;
use crate::core::{Engine, engine};
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::UrlRequest;
use rs_protocol::network::game::server::url_open::UrlOpen;
use rs_vm::ScriptError;

/// The links 2011's game interfaces offer (`openurl`): a website module, its
/// page and whether that is a signed-in one. `www` is the site itself, the
/// rest its services. From the advisor, quick chat (locked, and its
/// tutorial), the bug tracker (`snapshot_open_bugreport`, which only the
/// world runs), and the members' game warning's Subscribe Now and More Info.
const LINKS: [(&str, &str, bool); 6] = [
    ("forum", "forums.ws", false),
    ("www", "parents.ws", false),
    ("www", "kbase/viewcategory.ws?cat_id=827", false),
    ("bugtracker_v4", "index.html", false),
    ("billing_core", "paymentoptions.ws", true),
    ("weblogin", "members/members.ws", true),
];

/// The clan forum page the clan noticeboard and clan info link to, which a
/// clan's quick find code ends.
const CLAN_THREAD: &str = "threads.ws?threadid=";

/// The longest origin links take: the rest of `URL_OPEN`'s 255 bytes holds
/// the longest path.
const MAX_ORIGIN: usize = 200;

impl Engine {
    /// Has in-game links open on the first of `origins`, the website's
    /// addresses (`--site-origin`), and says whether it can: links need an
    /// `http` or `https` origin of at most 200 characters that 2011's client
    /// hands to the browser. Without one, a link opens nothing.
    pub fn set_site_origin(&mut self, origins: &str) -> bool {
        self.site_origin = link_origin(origins);
        self.site_origin.is_some()
    }
}

fn link_origin(origins: &str) -> Option<String> {
    let origin = origins.split_whitespace().next()?.trim_end_matches('/');
    let authority = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))?;
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    let valid = origin.len() <= MAX_ORIGIN
        && !host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-.".contains(&b))
        && port.is_none_or(|port| {
            port.bytes().all(|b| b.is_ascii_digit())
                && port.parse::<u16>().is_ok_and(|port| port > 0)
        });
    valid.then(|| origin.to_owned())
}

/// Handles the `UrlRequest` client protocol message.
///
/// A link in an interface: `openurl` sends its module, page and whether it is
/// a signed-in page, and the world answers with the page's address on the
/// website as `URL_OPEN`, one link a tick. A `www` page is on the site itself
/// and a service's under `m=<module>/`, as on Jagex's services hosts. A link
/// no game interface offers comes from a modified client.
impl ClientGameHandler for UrlRequest {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let Some(path) = site_path(&self.url, &self.text, self.mode) else {
            return Err(ScriptError::Client(format!(
                "Link no interface offers: {:?} {:?} {}",
                self.url, self.text, self.mode
            )));
        };
        let engine = engine();
        if active.last_link == Some(engine.clock) {
            return Ok(());
        }
        let Some(origin) = &engine.site_origin else {
            return Ok(());
        };
        active.last_link = Some(engine.clock);
        active.write(UrlOpen {
            url: &format!("{origin}/{path}"),
        });
        Ok(())
    }
}

/// The path on the website a link opens, `None` for one no game interface
/// offers. The client upper-cases a quick find code with no locale, so a
/// Turkish one sends its digit i as İ, which isn't CP-1252: '?'.
fn site_path(module: &str, page: &str, secure: u8) -> Option<String> {
    let secure = match secure {
        0 => false,
        1 => true,
        _ => return None,
    };
    if (module, secure) == ("clan-forum", false) {
        let code = page.strip_prefix(CLAN_THREAD)?.replace('?', "I");
        return is_qfc(&code).then(|| format!("m={module}/{CLAN_THREAD}{code}"));
    }
    LINKS
        .contains(&(module, page, secure))
        .then(|| match module {
            "www" => page.to_owned(),
            _ => format!("m={module}/{page}"),
        })
}

/// A quick find code as `clanforumqfc_tostring` writes one: a `long` in base
/// 36, upper case. A clan with none has no thread to link.
fn is_qfc(code: &str) -> bool {
    code.bytes()
        .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
        && (code == "0" || !code.starts_with('0'))
        && i64::from_str_radix(code, 36).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What 2011's client passes to the browser on Windows (`Signlink`'s
    /// request 16); it opens a URL with any other character only in an applet.
    const CLIENT_URL_CHARS: &str =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789?&=,.%+-_#:/*";

    #[test]
    fn every_link_opens_its_page_in_one_url_open() {
        let thread = format!("{CLAN_THREAD}1A2B");
        let longest = format!("{CLAN_THREAD}1Y2P0IJ32E8E7");
        assert_eq!(i64::from_str_radix("1Y2P0IJ32E8E7", 36), Ok(i64::MAX));
        let opened = [
            ("forum", "forums.ws", 0, "m=forum/forums.ws"),
            ("www", "parents.ws", 0, "parents.ws"),
            (
                "www",
                "kbase/viewcategory.ws?cat_id=827",
                0,
                "kbase/viewcategory.ws?cat_id=827",
            ),
            (
                "bugtracker_v4",
                "index.html",
                0,
                "m=bugtracker_v4/index.html",
            ),
            (
                "billing_core",
                "paymentoptions.ws",
                1,
                "m=billing_core/paymentoptions.ws",
            ),
            (
                "weblogin",
                "members/members.ws",
                1,
                "m=weblogin/members/members.ws",
            ),
            (
                "clan-forum",
                thread.as_str(),
                0,
                "m=clan-forum/threads.ws?threadid=1A2B",
            ),
            (
                "clan-forum",
                "threads.ws?threadid=1Y2P0?J32E8E7",
                0,
                "m=clan-forum/threads.ws?threadid=1Y2P0IJ32E8E7",
            ),
        ];
        for (module, page, secure, path) in opened {
            assert_eq!(site_path(module, page, secure).as_deref(), Some(path));
        }

        let origin = format!("https://{}", "a".repeat(MAX_ORIGIN - 8));
        let paths = LINKS
            .iter()
            .map(|&(module, page, secure)| site_path(module, page, secure as u8))
            .chain([site_path("clan-forum", &longest, 0)]);
        for path in paths {
            let url = format!("{origin}/{}", path.unwrap());
            assert!(url.len() <= u8::MAX as usize, "{url} fits URL_OPEN");
            assert!(
                url.chars().all(|c| CLIENT_URL_CHARS.contains(c)),
                "the client opens {url}"
            );
        }
    }

    #[test]
    fn a_link_no_interface_offers_opens_nothing() {
        for (module, page, secure) in [
            ("forum", "forums.ws", 1),
            ("billing_core", "paymentoptions.ws", 0),
            ("forum", "forums.ws", 2),
            ("www", "forums.ws", 0),
            ("forum", "parents.ws", 0),
            ("WWW", "parents.ws", 0),
            ("www", "/parents.ws", 0),
            ("www", "parents.ws?x=1", 0),
            ("http://example.com", "", 0),
            ("clan-forum", "threads.ws?threadid=", 0),
            ("clan-forum", "threads.ws?threadid=1a2b", 0),
            ("clan-forum", "threads.ws?threadid=01A", 0),
            ("clan-forum", "threads.ws?threadid=-1A", 0),
            ("clan-forum", "threads.ws?threadid=1Y2P0IJ32E8E8", 0),
            ("clan-forum", "threads.ws?threadid=1A&x=1", 0),
            ("clan-forum", "threads.ws?threadid=1A", 1),
            ("clan-forum", "forums.ws", 0),
        ] {
            assert_eq!(
                site_path(module, page, secure),
                None,
                "{module} {page} {secure}"
            );
        }
        assert_eq!(
            site_path("clan-forum", "threads.ws?threadid=0", 0).as_deref(),
            Some("m=clan-forum/threads.ws?threadid=0")
        );
    }

    #[test]
    fn links_open_on_the_first_website() {
        let first = "https://rustcity.org/ https://www.rustcity.org";
        assert_eq!(link_origin(first).as_deref(), Some("https://rustcity.org"));
        assert_eq!(
            link_origin("http://localhost:65535").as_deref(),
            Some("http://localhost:65535")
        );
        let longest = format!("https://{}", "a".repeat(MAX_ORIGIN - 8));
        assert_eq!(link_origin(&longest), Some(longest.clone()));
        let too_long = format!("{longest}a");
        for refused in [
            "",
            " ",
            "rustcity.org",
            "ftp://rustcity.org",
            "https://",
            "https://rustcity.org/news",
            "https://rustcity.org?x",
            "https://user@rustcity.org",
            "https://[::1]:8000",
            "https://:8000",
            "https://rustcity.org:",
            "https://rustcity.org:0",
            "https://rustcity.org:+80",
            "https://rustcity.org:abc",
            "https://a:1:2",
            "http://localhost:80800",
            too_long.as_str(),
        ] {
            assert_eq!(link_origin(refused), None, "{refused:?}");
        }
    }
}
