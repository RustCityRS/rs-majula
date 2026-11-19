use crate::player::{Disconnect, LobbyPlayer};
use crate::script::{LobbySession, LobbyVm};
use anyhow::{Result, anyhow, bail, ensure};
use rs_db::{
    AccountDetails, Claim, Db, Error as DbError, LobbyVisit, LobbyWorld, LoginAdmission, Passwords,
    Sanctions, lobby_node,
};
use rs_engine::{
    AuthResult, DB_FENCE, EtherInbound, EtherOutbound, LoginRequest, MAX_AUTH_RUNNING,
    MAX_AUTH_WAITING, MAX_PLAYING_PER_IP, TAKEOVER_AFTER_SECS, authenticate, max_friends_cap,
    world_list,
};
use rs_pack::startup::LobbyAssets;
use rs_protocol::LoginResponse;
use rs_protocol::login::{LobbyAccount, PLAYER_MOD_LEVEL, lobby_login_reply};
use rs_protocol::network::game::server::logout_full::LogoutFull;
use rs_protocol::world_list::WorldList;
use rs_util::base37::to_userhash;
use rs_vm::session::PlayerVars;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::util::RUNEDAY_EPOCH_DAYS;
use std::collections::{HashMap, HashSet};
use std::mem::take;
use std::net::{IpAddr, Ipv4Addr};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{
    Semaphore,
    mpsc::{UnboundedReceiver, UnboundedSender, error::TryRecvError},
};
use tokio::task::{Id, JoinSet};
use tokio::time::{Instant as TokioInstant, MissedTickBehavior, interval, sleep, timeout_at};
use tokio::try_join;
use tracing::{error, info, warn};

const TICK: Duration = Duration::from_millis(50);
const LOGIN_TIMEOUT: Duration = Duration::from_secs(6);
const STATUS_INTERVAL: Duration = Duration::from_secs(5);
const RELEASE_RETRY: Duration = Duration::from_secs(1);

type Visit = (LobbyVisit, IpAddr, Option<u8>);

pub struct LobbyConfig {
    pub id: u8,
    pub instance: i64,
    pub revision: u16,
    pub capacity: usize,
    pub auto_register: bool,
    pub block_multilog: bool,
    pub passwords: Arc<Passwords>,
}

struct Pending {
    request: LoginRequest,
    started: Instant,
    attempt: u64,
    cancelled: bool,
    staff: u8,
    unread: u16,
    member: bool,
    jcoins: i32,
    made_day: u16,
    last_login_day: u16,
    last_login_ip: Ipv4Addr,
    play_age: i32,
    account: Option<AccountDetails>,
    stage: Stage,
}

/// The content's vars that show an account's email and Jagex Account
/// Guardian in the lobby, where it has them (742).
#[derive(Clone, Copy)]
struct AccountVars {
    email: Option<u16>,
    guardian: Option<u16>,
}

#[derive(PartialEq)]
enum Stage {
    Authenticating,
    Checking,
    Retry(Instant),
    Claiming,
}

enum Reply {
    Auth(u64, u64, AuthResult, Option<AccountDetails>),
    Claim(u64, u64, Option<Claim>),
    Left(u64),
    StatusUnavailable(DbError),
    Status(Instant, Vec<LobbyWorld>, Vec<Sanctions>),
}

#[derive(Debug)]
enum Job {
    Auth(u64, u64),
    Claim(u64, u64),
    Release(u64, Option<Visit>),
    Status,
}

pub struct LobbyEngine {
    config: LobbyConfig,
    db: Db,
    assets: LobbyAssets,
    vm: LobbyVm,
    ether: UnboundedSender<EtherOutbound>,
    ether_rx: UnboundedReceiver<EtherInbound>,
    logins: UnboundedReceiver<LoginRequest>,
    ready: bool,
    lease: Instant,
    next_status: Instant,
    status_running: bool,
    pending: HashMap<u64, Pending>,
    players: HashMap<u64, LobbyPlayer>,
    leaving: HashSet<u64>,
    jobs: JoinSet<Result<Reply>>,
    running: HashMap<Id, Job>,
    hashing: Arc<Semaphore>,
    next_attempt: u64,
    auth_jobs: usize,
    worlds: WorldList,
    preferred: Option<(u16, String)>,
    account_vars: AccountVars,
}

impl LobbyEngine {
    pub async fn start(
        config: LobbyConfig,
        db: Db,
        assets: LobbyAssets,
        ether: UnboundedSender<EtherOutbound>,
        ether_rx: UnboundedReceiver<EtherInbound>,
        logins: UnboundedReceiver<LoginRequest>,
    ) -> Result<Self> {
        let lease = Instant::now();
        let node = lobby_node(config.id);
        let worlds = db
            .lobby_worlds(config.revision.into(), TAKEOVER_AFTER_SECS)
            .await?;
        ensure!(
            db.start_lobby(node, config.instance, TAKEOVER_AFTER_SECS)
                .await?,
            "lobby {} still has a live database lease",
            config.id
        );
        let account_vars = AccountVars {
            email: assets
                .varps
                .get_by_debugname("email_validation")
                .and_then(|varp| u16::try_from(varp.id.index()).ok()),
            guardian: assets
                .varpbits
                .get_by_debugname("account_guardian")
                .and_then(|bit| u16::try_from(bit.id.index()).ok()),
        };
        let mut engine = Self {
            config,
            db,
            vm: LobbyVm::new(&assets),
            assets,
            ether,
            ether_rx,
            logins,
            ready: false,
            lease,
            next_status: lease + STATUS_INTERVAL,
            status_running: false,
            pending: HashMap::new(),
            players: HashMap::new(),
            leaving: HashSet::new(),
            jobs: JoinSet::new(),
            running: HashMap::new(),
            hashing: Arc::new(Semaphore::new(MAX_AUTH_RUNNING)),
            next_attempt: 0,
            auth_jobs: 0,
            worlds: world_list(Vec::new()),
            preferred: None,
            account_vars,
        };
        engine.set_worlds(worlds);
        Ok(engine)
    }

    fn node(&self) -> i16 {
        lobby_node(self.config.id)
    }

    /// Sets the content's email and Guardian vars from the account, before
    /// its login script runs.
    fn show_account(&self, player: &mut LobbyPlayer, account: &AccountDetails) {
        let mut session = LobbySession {
            player,
            assets: &self.assets,
        };
        let mut vars = PlayerVars {
            player: &mut session,
            varps: &self.assets.varps,
            bits: &self.assets.varpbits,
        };
        let set = [
            self.account_vars
                .email
                .map(|id| vars.set_varp(id, (account.email_status == 3).into())),
            self.account_vars
                .guardian
                .map(|id| vars.set_varbit(id, account.account_guardian.into())),
        ];
        for err in set.into_iter().flatten().filter_map(Result::err) {
            error!("Lobby couldn't set an account var: {err}");
        }
    }

    fn set_worlds(&mut self, worlds: Vec<LobbyWorld>) {
        self.preferred = worlds.first().map(|w| (w.id as u16, w.address.clone()));
        self.worlds = world_list(worlds);
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut ticks = interval(TICK);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticks.tick().await;
            let now = Instant::now();
            ensure!(
                now.duration_since(self.lease) < DB_FENCE,
                "lobby database lease expired"
            );
            for _ in 0..8192 {
                match self.ether_rx.try_recv() {
                    Ok(msg) => self.on_ether(msg),
                    Err(TryRecvError::Empty) => break,
                    Err(_) => bail!("lobby Ether channel closed"),
                }
            }
            while let Some(joined) = self.jobs.try_join_next_with_id() {
                match joined {
                    Ok((task, reply)) => {
                        self.running.remove(&task);
                        self.complete(reply?)?;
                    }
                    Err(err) => {
                        let job = self.running.remove(&err.id());
                        error!("Lobby job {job:?} failed: {err}");
                        if let Some(job) = job {
                            self.failed(job)?;
                        }
                    }
                }
            }
            if now >= self.next_status && !self.status_running {
                self.refresh();
                self.next_status = now + STATUS_INTERVAL;
            }
            let mut left = Vec::new();
            for (&name, player) in &mut self.players {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    player.tick(now, &self.assets, &self.vm, &self.worlds)
                }));
                match result {
                    Ok(Some(reason)) => left.push((name, matches!(reason, Disconnect::Closed))),
                    Ok(None) => {}
                    Err(_) => {
                        error!("Lobby player {name} failed during input");
                        left.push((name, false));
                    }
                }
            }
            for (name, visit) in left {
                self.leave(name, visit, None);
            }
            let mut expired = Vec::new();
            for (&name, pending) in &mut self.pending {
                if now.duration_since(pending.started) >= LOGIN_TIMEOUT {
                    match pending.stage {
                        Stage::Claiming => {}
                        Stage::Retry(_) => expired.push((name, LoginResponse::Duplicate)),
                        _ => expired.push((name, LoginResponse::BadSave)),
                    }
                } else if matches!(pending.stage, Stage::Retry(at) if now >= at) {
                    pending.stage = Stage::Checking;
                    let _ = self.ether.send(EtherOutbound::LoginCheck {
                        user37: name,
                        max_per_ip: MAX_PLAYING_PER_IP,
                        ip: Some(pending.request.remote_addr.ip()),
                    });
                }
            }
            for (name, code) in expired {
                self.reject(name, code);
            }
            for _ in 0..128 {
                let Ok(request) = self.logins.try_recv() else {
                    break;
                };
                self.login(request);
            }
        }
    }

    fn login(&mut self, mut request: LoginRequest) {
        let name = to_userhash(&request.username);
        let refusal = if !self.ready || self.lease.elapsed() >= DB_FENCE {
            Some(LoginResponse::LoginServerOffline)
        } else if self.players.contains_key(&name)
            || self.leaving.contains(&name)
            || self.pending.contains_key(&name)
        {
            Some(LoginResponse::Duplicate)
        } else if self.players.len() + self.pending.len() >= self.config.capacity {
            Some(LoginResponse::ServerFull)
        } else if self.pending.len().max(self.auth_jobs) >= MAX_AUTH_RUNNING + MAX_AUTH_WAITING {
            Some(LoginResponse::LoginServerOffline)
        } else {
            None
        };
        if let Some(code) = refusal {
            request.handle.send(vec![code as u8]);
            return;
        }
        let attempt = self.next_attempt;
        self.next_attempt = attempt.wrapping_add(1);
        let db = self.db.clone();
        let passwords = self.config.passwords.clone();
        let hashing = self.hashing.clone();
        let node = self.node();
        let (register, block) = (self.config.auto_register, self.config.block_multilog);
        let ip = request.remote_addr.ip();
        let uid = request.uid;
        let device = take(&mut request.device);
        let password = take(&mut request.password);
        self.auth_jobs += 1;
        let deadline = TokioInstant::now() + LOGIN_TIMEOUT;
        self.spawn(Job::Auth(name, attempt), async move {
            let Ok(Ok(_permit)) = timeout_at(deadline, hashing.acquire_owned()).await else {
                return Ok(Reply::Auth(name, attempt, AuthResult::Busy, None));
            };
            let result = authenticate(
                &db, &passwords, register, block, node, name, &password, ip, uid, &device,
            )
            .await
            .unwrap_or_else(|e| {
                error!("Lobby authentication failed for {name}: {e}");
                AuthResult::Busy
            });
            let account = if matches!(result, AuthResult::Accepted { .. }) {
                db.lobby_account(name).await.unwrap_or_else(|e| {
                    error!("Lobby couldn't read the account of {name}: {e}");
                    None
                })
            } else {
                None
            };
            Ok(Reply::Auth(name, attempt, result, account))
        });
        self.pending.insert(
            name,
            Pending {
                request,
                attempt,
                cancelled: false,
                started: Instant::now(),
                staff: 0,
                unread: 0,
                member: false,
                jcoins: 0,
                made_day: 0,
                last_login_day: 0,
                last_login_ip: Ipv4Addr::UNSPECIFIED,
                play_age: 0,
                account: None,
                stage: Stage::Authenticating,
            },
        );
    }

    fn reject(&mut self, name: u64, code: LoginResponse) {
        if let Some(mut p) = self.pending.remove(&name) {
            if p.stage == Stage::Claiming {
                let _ = self.ether.send(EtherOutbound::LoginAbort {
                    user37: name,
                    ip: Some(p.request.remote_addr.ip()),
                });
            }
            p.request.handle.send(vec![code as u8]);
        }
    }

    fn complete(&mut self, reply: Reply) -> Result<()> {
        match reply {
            Reply::Auth(name, id, result, account) => {
                self.auth_jobs -= 1;
                let Some(p) = self.pending.get_mut(&name).filter(|p| p.attempt == id) else {
                    return Ok(());
                };
                match result {
                    AuthResult::Accepted {
                        staff_mod_level,
                        account_age,
                        unread_messages,
                        last_login,
                        last_login_ip,
                        member,
                        jcoins,
                        ..
                    } if self.ready && p.started.elapsed() < LOGIN_TIMEOUT && account.is_some() => {
                        p.staff = staff_mod_level;
                        p.unread = unread_messages;
                        p.member = member;
                        p.jcoins = jcoins;
                        p.made_day = runeday(
                            SystemTime::now()
                                .checked_sub(account_age)
                                .and_then(|made| made.duration_since(UNIX_EPOCH).ok())
                                .map(|since| (since.as_secs() / 86_400) as i64),
                        );
                        if let (Some(at), Some(ip)) = (last_login, last_login_ip) {
                            p.last_login_day = runeday(
                                at.duration_since(UNIX_EPOCH)
                                    .ok()
                                    .map(|since| (since.as_secs() / 86_400) as i64),
                            );
                            p.last_login_ip = ip;
                        }
                        p.play_age = (account_age.as_secs() / 60).clamp(1, i32::MAX as u64) as i32;
                        p.account = account;
                        p.stage = Stage::Checking;
                        let _ = self.ether.send(EtherOutbound::LoginCheck {
                            user37: name,
                            max_per_ip: MAX_PLAYING_PER_IP,
                            ip: Some(p.request.remote_addr.ip()),
                        });
                    }
                    AuthResult::InvalidCredentials => {
                        self.reject(name, LoginResponse::BadCredentials)
                    }
                    AuthResult::Banned => self.reject(name, LoginResponse::Banned),
                    AuthResult::MultiLogging => self.reject(name, LoginResponse::IpLimit),
                    AuthResult::TooManyAttempts => {
                        self.reject(name, LoginResponse::TooManyAttempts)
                    }
                    _ => self.reject(name, LoginResponse::LoginServerOffline),
                }
            }
            Reply::Claim(name, id, claim) => {
                let Some(p) = self.pending.get(&name).filter(|p| p.attempt == id) else {
                    bail!("lobby claim completed without its login");
                };
                match claim {
                    Some(Claim::Claimed { .. }) => {
                        if p.cancelled
                            || !self.ready
                            || self.lease.elapsed() >= DB_FENCE
                            || p.started.elapsed() >= LOGIN_TIMEOUT
                            || p.request.handle.inbox.is_closed()
                        {
                            self.reject(name, LoginResponse::LoginServerOffline);
                            self.release(name, None);
                        } else {
                            let p = self.pending.remove(&name).unwrap();
                            let account = p.account.as_ref().unwrap();
                            let world = self
                                .preferred
                                .as_ref()
                                .map(|(id, addr)| (*id, addr.as_str()));
                            let reply = lobby_login_reply(&reply_account(&p, account, world));
                            let ip = p.request.remote_addr.ip();
                            let mut player = LobbyPlayer::new(
                                p.request.handle,
                                name,
                                ip,
                                p.request.client_session,
                                self.assets.toplevel,
                            );
                            player.member = p.member;
                            self.show_account(&mut player, account);
                            let started = catch_unwind(AssertUnwindSafe(|| {
                                player.run_trigger(
                                    &self.assets,
                                    &self.vm,
                                    ServerTriggerType::Login,
                                    None,
                                )
                            }));
                            if !matches!(started, Ok(Ok(true))) {
                                error!("Lobby login script failed for {name}: {started:?}");
                                player
                                    .handle
                                    .finish_lobby_login(vec![LoginResponse::BadSave as u8], false);
                                let _ = self.ether.send(EtherOutbound::LoginAbort {
                                    user37: name,
                                    ip: Some(ip),
                                });
                                self.release(name, None);
                                return Ok(());
                            }
                            player.handle.finish_lobby_login(reply, true);
                            let member = player.member;
                            self.players.insert(name, player);
                            let _ = self.ether.send(EtherOutbound::PlayerLogin {
                                user37: name,
                                pid: 0,
                                max_friends: max_friends_cap(member),
                                ip: Some(ip),
                            });
                        }
                    }
                    Some(Claim::Busy { .. }) => self.reject(name, LoginResponse::Duplicate),
                    Some(Claim::IpLimit) => self.reject(name, LoginResponse::IpLimit),
                    Some(_) => self.reject(name, LoginResponse::BadSession),
                    None => {
                        self.reject(name, LoginResponse::LoginServerOffline);
                        self.release(name, None);
                    }
                }
            }
            Reply::Left(name) => {
                self.leaving.remove(&name);
                let _ = self
                    .ether
                    .send(EtherOutbound::PlayerLogout { user37: name });
            }
            Reply::StatusUnavailable(err) => {
                self.status_running = false;
                warn!("Lobby heartbeat check failed: {err}");
            }
            Reply::Status(at, worlds, sanctions) => {
                self.lease = at;
                self.status_running = false;
                self.set_worlds(worlds);
                for s in sanctions.into_iter().filter(|s| s.banned) {
                    if let Some(p) = self.players.get_mut(&s.name_hash) {
                        p.handle.write(LogoutFull);
                    }
                    self.leave(s.name_hash, false, None);
                }
            }
        }
        Ok(())
    }

    fn on_ether(&mut self, msg: EtherInbound) {
        match msg {
            EtherInbound::EtherDisconnected | EtherInbound::EtherReconnected => {
                self.ready = matches!(msg, EtherInbound::EtherReconnected);
                let names: Vec<_> = self.pending.keys().copied().collect();
                for name in names {
                    if self.pending[&name].stage == Stage::Claiming {
                        self.pending.get_mut(&name).unwrap().cancelled = true;
                    } else {
                        self.reject(name, LoginResponse::LoginServerOffline);
                    }
                }
                if self.ready {
                    for (&user37, p) in &self.players {
                        let _ = self.ether.send(EtherOutbound::PlayerResync {
                            user37,
                            pid: 0,
                            private_mode: 0,
                            max_friends: max_friends_cap(p.member),
                            ip: Some(p.ip),
                        });
                    }
                    let _ = self.ether.send(EtherOutbound::RefreshAll);
                }
            }
            EtherInbound::LoginCheckResponse {
                user37,
                allowed,
                ip_limited,
                ..
            } => {
                let node = self.node();
                let Some(p) = self
                    .pending
                    .get_mut(&user37)
                    .filter(|p| p.stage == Stage::Checking)
                else {
                    return;
                };
                if !self.ready {
                    self.reject(user37, LoginResponse::LoginServerOffline);
                    return;
                }
                if ip_limited {
                    self.reject(user37, LoginResponse::IpLimit);
                    return;
                }
                if !allowed {
                    p.stage = Stage::Retry(Instant::now() + Duration::from_millis(300));
                    return;
                }
                p.stage = Stage::Claiming;
                let db = self.db.clone();
                let id = p.attempt;
                let admission = LoginAdmission {
                    ip: p.request.remote_addr.ip(),
                    max_per_ip: MAX_PLAYING_PER_IP,
                    allow_direct: false,
                    lobby_instance: Some(self.config.instance),
                    visit: Some(LobbyVisit {
                        client_session: p.request.client_session,
                        revision: self.config.revision.into(),
                    }),
                };
                self.spawn(Job::Claim(user37, id), async move {
                    let claim = db
                        .claim_login(user37, node, TAKEOVER_AFTER_SECS, Some(admission))
                        .await
                        .inspect_err(|e| error!("Lobby claim failed for {user37}: {e}"))
                        .ok();
                    Ok(Reply::Claim(user37, id, claim))
                });
            }
            EtherInbound::LobbyTransfer {
                user37,
                client_session,
                ip,
                world,
                revision,
            } if revision == self.config.revision
                && self.players.get(&user37).is_some_and(|p| {
                    p.client_session == client_session && p.ip.to_canonical() == ip.to_canonical()
                }) =>
            {
                self.leave(user37, true, Some(world));
            }
            _ => {}
        }
    }

    fn leave(&mut self, name: u64, visit: bool, world: Option<u8>) {
        let Some(mut p) = self.players.remove(&name) else {
            return;
        };
        let _ = catch_unwind(AssertUnwindSafe(|| {
            p.trigger(&self.assets, &self.vm, ServerTriggerType::Logout, None)
        }));
        let visit = visit.then_some((
            LobbyVisit {
                client_session: p.client_session,
                revision: self.config.revision.into(),
            },
            p.ip,
            world,
        ));
        self.release(name, visit);
    }

    fn release(&mut self, name: u64, visit: Option<Visit>) {
        self.leaving.insert(name);
        self.spawn_release(name, visit, false);
    }

    fn spawn_release(&mut self, name: u64, visit: Option<Visit>, delay: bool) {
        let db = self.db.clone();
        let node = self.node();
        let instance = self.config.instance;
        self.spawn(Job::Release(name, visit), async move {
            if delay {
                sleep(RELEASE_RETRY).await;
            }
            while let Err(err) = db.leave_lobby(name, node, instance, visit).await {
                warn!("Lobby release failed for {name}: {err}");
                sleep(RELEASE_RETRY).await;
            }
            Ok(Reply::Left(name))
        });
    }

    fn spawn(&mut self, job: Job, task: impl Future<Output = Result<Reply>> + Send + 'static) {
        let id = self.jobs.spawn(task).id();
        self.running.insert(id, job);
    }

    fn failed(&mut self, job: Job) -> Result<()> {
        match job {
            Job::Auth(name, id) => self.complete(Reply::Auth(name, id, AuthResult::Busy, None))?,
            Job::Claim(name, id) => self.complete(Reply::Claim(name, id, None))?,
            Job::Release(name, visit) => self.spawn_release(name, visit, true),
            Job::Status => self.status_running = false,
        }
        Ok(())
    }

    fn refresh(&mut self) {
        let db = self.db.clone();
        let (node, instance, revision) = (self.node(), self.config.instance, self.config.revision);
        let names: Vec<_> = self.players.keys().copied().collect();
        self.status_running = true;
        self.spawn(Job::Status, async move {
            let at = Instant::now();
            match db
                .renew_lobby(node, instance, names.len() as i32, TAKEOVER_AFTER_SECS)
                .await
            {
                Ok(true) => {}
                Ok(false) => return Err(anyhow!("lost lobby instance lease")),
                Err(err) => return Ok(Reply::StatusUnavailable(err)),
            }
            let (worlds, sanctions) = match try_join!(
                db.lobby_worlds(revision.into(), TAKEOVER_AFTER_SECS),
                db.sanctions(&names)
            ) {
                Ok(values) => values,
                Err(err) => return Ok(Reply::StatusUnavailable(err)),
            };
            Ok(Reply::Status(at, worlds, sanctions))
        });
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        for (_, mut p) in self.players.drain() {
            let _ = catch_unwind(AssertUnwindSafe(|| {
                p.trigger(&self.assets, &self.vm, ServerTriggerType::Logout, None)
            }));
            p.handle.write(LogoutFull);
        }
        self.pending.clear();
        self.jobs.shutdown().await;
        self.db
            .stop_lobby(self.node(), self.config.instance)
            .await?;
        info!("Lobby stopped");
        Ok(())
    }
}

/// The runeday `days` after 1970 falls on, never 0: the lobby reads 0 as
/// no day at all.
fn runeday(days: Option<i64>) -> u16 {
    days.map_or(0, |days| {
        (days - i64::from(RUNEDAY_EPOCH_DAYS)).clamp(1, u16::MAX.into()) as u16
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64)
}

/// An accepted login as its reply tells the client. An account with no date
/// of birth sends 0: 742 reads 8388605 as none recorded and holds lobby chat
/// until the player enters one, which this lobby can't take.
fn reply_account<'a>(
    p: &Pending,
    account: &'a AccountDetails,
    world: Option<(u16, &'a str)>,
) -> LobbyAccount<'a> {
    let members_until = account.members_until_ms.unwrap_or(0);
    LobbyAccount {
        display_name: &account.display_name,
        staff_mod_level: p.staff.min(2),
        player_mod_level: if p.staff == 1 { PLAYER_MOD_LEVEL } else { 0 },
        underage: account.underage,
        chat_consent: account.chat_consent,
        advert_consent: account.advert_consent,
        #[cfg(since_742)]
        date_of_birth: account
            .date_of_birth_days
            .map_or(0, |days| days - RUNEDAY_EPOCH_DAYS),
        #[cfg(since_742)]
        gender: account
            .gender
            .map_or(0, |gender| gender.clamp(0, 255) as u8),
        members_until,
        members_left: (members_until - now_ms()).max(0),
        member: p.member,
        subscribed: account.subscribed,
        jcoins: p.jcoins,
        #[cfg(since_742)]
        loyalty_points: account.loyalty_points,
        recovery_day: p.made_day,
        unread_messages: p.unread,
        last_login_day: p.last_login_day,
        last_login_ip: p.last_login_ip,
        email_status: account.email_status.clamp(0, 255) as u8,
        card_expiry_day: runeday(account.card_expires_days.map(i64::from)),
        grace_expiry_day: runeday(account.grace_expires_days.map(i64::from)),
        dob_requested: account.dob_requested,
        play_age: p.play_age,
        world,
    }
}
