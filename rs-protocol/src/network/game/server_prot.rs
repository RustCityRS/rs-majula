macro_rules! server_prot {
    ( $( $variant:ident = $id:expr ),* $(,)? ) => {
        #[repr(u8)]
        #[derive(Debug)]
        pub enum ServerProt {
            $( $variant = $id, )*
        }
    };
}

#[cfg(rev = "225")]
server_prot! {
    NpcInfo = 1, // NXT naming
    IfSetColour = 2, // NXT naming
    CamMoveTo = 3, // NXT naming
    MessageGame = 4, // NXT naming
    UpdateZonePartialFollows = 7, // NXT naming
    SynthSound = 12, // NXT naming
    CamShake = 13, // NXT naming
    IfOpenChat = 14,
    UpdateInvStopTransmit = 15, // NXT naming
    UnsetMapFlag = 19, // NXT has "SET_MAP_FLAG" but we cannot control the position
    DataLocDone = 20,
    UpdateIgnoreList = 21, // NXT naming
    UpdateRunWeight = 22, // NXT naming
    LocMerge = 23,
    HintArrow = 25, // NXT naming
    IfSetHide = 26, // NXT naming
    IfOpenMainSide = 28,
    ChatFilterSettings = 32, // NXT naming
    MessagePrivate = 41, // NXT naming
    LocAnim = 42,
    UpdateRebootTimer = 43, // NXT naming
    UpdateStat = 44, // NXT naming
    IfSetObject = 46, // NXT naming
    ObjDel = 49,
    ObjReveal = 50,
    MidiSong = 54, // NXT naming
    LocAddChange = 59,
    UpdateRunEnergy = 68, // NXT naming
    MapProjAnim = 69,
    CamLookAt = 74, // NXT naming
    LocDel = 76,
    DataLandDone = 80,
    IfSetTabActive = 84,
    IfSetModel = 87, // NXT naming
    UpdateInvFull = 98, // NXT naming
    TutFlash = 126,
    IfClose = 129,
    IfSetRecol = 103, // NXT naming
    DataLand = 132,
    FinishTracking = 133,
    UpdateZoneFullFollows = 135, // NXT naming
    ResetAnims = 136, // NXT naming
    UpdatePid = 139,
    LastLoginInfo = 140, // NXT naming
    Logout = 142, // NXT naming
    IfSetAnim = 146, // NXT naming
    VarpSmall = 150, // NXT naming
    ObjCount = 151,
    UpdateFriendList = 152, // NXT naming
    UpdateZonePartialEnclosed = 162, // NXT naming
    IfSetTab = 167,
    IfOpenMain = 168,
    VarpLarge = 175, // NXT naming
    PlayerInfo = 184, // NXT naming
    TutOpen = 185,
    MapAnim = 191,
    ResetClientVarCache = 193, // NXT naming
    IfOpenSide = 195,
    IfSetPlayerHead = 197, // NXT naming
    IfSetText = 201, // NXT naming
    IfSetNpcHead = 204, // NXT naming
    IfSetPosition = 209, // NXT naming
    MidiJingle = 212, // NXT naming
    UpdateInvPartial = 213, // NXT naming
    DataLoc = 220,
    ObjAdd = 223,
    EnableTracking = 226,
    RebuildNormal = 237, // NXT naming (do we really need _normal if there's no region rebuild?)
    CamReset = 239, // NXT naming
    PCountDialog = 243, // named after runescript command + client resume_p_countdialog packet
    SetMultiway = 254,
}

#[cfg(since_244)]
server_prot! {
    UpdateIgnoreList = 7,
    ChatFilterSettings = 9,
    IfOpenMain = 10,
    CamMoveTo = 12,
    Logout = 17,
    EnableTracking = 22,
    UpdateStat = 24,
    LocMerge = 29,
    MessagePrivate = 30,
    ObjDel = 39,
    LastLoginInfo = 44,
    HintArrow = 49,
    CamShake = 50,
    CamReset = 53,
    IfSetTabActive = 56,
    FinishTracking = 60,
    UnsetMapFlag = 62,
    ObjReveal = 69,
    UpdateFriendList = 70,
    UpdateInvFull = 72,
    IfSetColour = 78,
    UpdateRebootTimer = 85,
    PlayerInfo = 86,
    ResetClientVarCache = 87,
    UpdateZonePartialFollows = 94,
    MessageGame = 95,
    SetMultiway = 97,
    IfSetRecol = 103,
    IfSetPlayerHead = 108,
    IfSetHide = 123,
    LocDel = 125,
    IfSetNpcHead = 129,
    UpdateZoneFullFollows = 131,
    UpdateInvPartial = 132,
    MapProjAnim = 137,
    SynthSound = 151,
    PCountDialog = 152,
    IfSetText = 154,
    LocAnim = 155,
    UpdateRunWeight = 160,
    UpdateInvStopTransmit = 162,
    IfSetObject = 164,
    RebuildNormal = 165,
    TutFlash = 168,
    MidiJingle = 173,
    TutOpen = 174,
    IfOpenSide = 176,
    UpdateRunEnergy = 177,
    IfOpenChat = 189,
    MapAnim = 198,
    IfSetTab = 200,
    IfOpenMainSide = 207,
    ObjCount = 209,
    UpdatePid = 210,
    IfClose = 214,
    IfSetAnim = 219,
    CamLookAt = 222,
    VarpLarge = 226,
    LocAddChange = 232,
    UpdateZonePartialEnclosed = 233,
    ObjAdd = 234,
    VarpSmall = 236,
    MidiSong = 240,
    IfSetPosition = 241,
    ResetAnims = 242,
    NpcInfo = 244,
    IfSetModel = 245,
}
