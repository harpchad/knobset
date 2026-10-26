CREATE TABLE tabAlertToneFile (
    AlertToneID INTEGER NOT NULL,
    AlertToneFile VARCHAR( 30 ),
    AlertToneType INTEGER NOT NULL DEFAULT '0'
);
CREATE TABLE tabAllBandScanParam (
    AllBandScanSettingID INTEGER,
    StartFreq INTEGER,
    EndFreq INTEGER,
    StepSize INTEGER,
    BandWidth SMALLINT,
    StayTime INTEGER,
    RxPauseBetweenScanPaths INTEGER,
    VirtualChId INTEGER,
    AutoContinueTime INTEGER,
    SeqId INTEGER,
    RecvAllFlag INTEGER DEFAULT 1,
    EovTimeout INTEGER DEFAULT 1000,
    DMRCompatibility INTEGER NOT NULL DEFAULT 0,
    DMRVocoder INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tabBattery (
    CheckBeginPercent INTEGER,
    LowLevel INTEGER,
    VeryLowLevel INTEGER,
    CriticalLevel INTEGER,
    LowLevelInterval INTEGER,
    VeryLowLevelInterval INTEGER,
    CriticalLevelInterval INTEGER,
    InsideTimer INTEGER,
    OutsideTimer INTEGER,
    OutsideTimerOn INTEGER
);
CREATE TABLE tabBrightLevelToBrightness(
    BrightnessLevel INTEGER NOT NULL,
    Brightness INTEGER NOT NULL,
    PRIMARY KEY ( BrightnessLevel )
);
CREATE TABLE tabCallAlertGroup (
    ZoneKnobID        INTEGER        NOT NULL,
    SUID             INTEGER        NOT NULL,
    Address          VARCHAR( 30 )  NOT NULL,
    CallAlertTimeSec INTEGER  NOT NULL,
    CallAlertTimeMSec INTEGER NOT NULL
);
CREATE TABLE tabCannedMessageContent (
    MessageID  INTEGER  NOT NULL,
    MessageContent  VARCHAR( 250 ),
    PRIMARY KEY ( MessageID )
);
CREATE TABLE tabChannel (
    ZoneKnobID INTEGER NOT NULL,
    RecvMode INTEGER NOT NULL,
    RXFrequency INTEGER NOT NULL,
    RxChanBandWidth INTEGER DEFAULT 1 NOT NULL,
    isSupportRepeater INTEGER DEFAULT 0 NOT NULL,
    CarrierSquelchSetting INTEGER DEFAULT 80 NOT NULL,
    SINADSquelchSetting INTEGER DEFAULT 6 NOT NULL,
    SignallingSquelchLogic INTEGER DEFAULT 0 NOT NULL,
    AutoMute INTEGER DEFAULT 0 NOT NULL,
    Emphasis INTEGER DEFAULT 0 NOT NULL,
    Compander INTEGER DEFAULT 0 NOT NULL,
    SideTone INTEGER DEFAULT 0 NOT NULL,
    RFActivityTimeout INTEGER DEFAULT 500 NOT NULL,
    UniLongDataEnable INTEGER DEFAULT 1 NOT NULL,
    PowerSavingRxOffDur INTEGER DEFAULT 0 NOT NULL,
    NumOfSA INTEGER DEFAULT 0 NOT NULL,
    SAType1 INTEGER DEFAULT 0 NOT NULL,
    CTCD1 INTEGER DEFAULT 0 NOT NULL,
    CDModeSense1 INTEGER DEFAULT 0 NOT NULL,
	SA1Sig VARCHAR(16) DEFAULT NULL,
    SAType2 INTEGER DEFAULT 0 NOT NULL,
    CTCD2 INTEGER DEFAULT 0 NOT NULL,
    CDModeSense2 INTEGER DEFAULT 0 NOT NULL,
	SA2Sig VARCHAR(16) DEFAULT NULL,
    SAType3 INTEGER DEFAULT 0 NOT NULL,
    CTCD3 INTEGER DEFAULT 0 NOT NULL,
    CDModeSense3 INTEGER DEFAULT 0 NOT NULL,
	SA3Sig VARCHAR(16) DEFAULT NULL,
    SAType4 INTEGER DEFAULT 0 NOT NULL,
    CTCD4 INTEGER DEFAULT 0 NOT NULL,
    CDModeSense4 INTEGER DEFAULT 0 NOT NULL,
	SA4Sig VARCHAR(16) DEFAULT NULL,
    SAType5 INTEGER DEFAULT 0 NOT NULL,
    CTCD5 INTEGER DEFAULT 0 NOT NULL,
    CDModeSense5 INTEGER DEFAULT 0 NOT NULL,
	SA5Sig VARCHAR(16) DEFAULT NULL,
    PRIMARY KEY (ZoneKnobID)
 );
CREATE TABLE tabCipherKeyAssignment (
    CHID  INTEGER  NOT NULL,
    KeyName  VARCHAR( 32 )  NOT NULL,
    CKR  INTEGER  NOT NULL,
    KeyID  INTEGER  NOT NULL,
    Type  INTEGER  NOT NULL,
    AssignmentID  INTEGER  NOT NULL,
    KeyStrappingOption  INTEGER  NOT NULL,
    SecureSwitch  INTEGER  NOT NULL,
    PRIMARY KEY ( CHID, Type, AssignmentID )
);
CREATE TABLE tabClockAlarmSetting (
    FuncID  INTEGER,
    AlarmTime  INTEGER,
    AlarmAlertType  INTEGER,
    AlarmOperationType  INTEGER,
    WeeklySetting  INTEGER,
    PRIMARY KEY ( FuncID )
);
CREATE TABLE tabContacts (
    ContactID INTEGER NOT NULL,
    SignalingType INTEGER NOT NULL,
    Address VARCHAR (30) NOT NULL,
    Alias VARCHAR (14) NOT NULL,
    AlertMode INTEGER NOT NULL,
    AlertToneIndex INTEGER NOT NULL,
    VoiceStorage INTEGER NOT NULL,
    RelayControl INTEGER NOT NULL,
    RelayOnTime INTEGER NOT NULL,
    BackgroundColor INTEGER NOT NULL,
    PRIMARY KEY (ContactID,SignalingType)
);
CREATE TABLE tabCurrentZoneAndLocked (
    CurrentZoneID INTEGER NOT NULL,
    LockedChannelID INTEGER NOT NULL
);
CREATE TABLE tabDMRSignaling (
    SigID INTEGER NOT NULL,
    TXPreFrames INTEGER DEFAULT 0,
    RxColorCode INTEGER DEFAULT 0,
    RptSlot INTEGER,
    RptTiming INTEGER,
    RCRxEnable INTEGER,
    NoSigActTimeout INTEGER NOT NULL,
    ACKWaitTimeout INTEGER NOT NULL,
    MFID INTEGER,
    SAP4NRS INTEGER DEFAULT 4,
    SAP4RS INTEGER DEFAULT 5,
    DMRVocoder INTEGER NOT NULL DEFAULT 0,
    DMRCompatibility INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY ( SigID )
);
CREATE TABLE tabDSPEngineerSettings (
    Enable     SMALLINT,
    Parameters VARCHAR( 128 )
);
CREATE TABLE tabDetailScreenItem (
    Style           SMALLINT,
    RecvMode        SMALLINT,
    Item            SMALLINT,
    ItemIndex       SMALLINT,
    ContentColor    VARCHAR( 128 ),
    FontSize        VARCHAR( 30 ),
    BackgroundColor VARCHAR( 7 ),
    LineHeight      SMALLINT,
    Alignment       VARCHAR( 30 )
);
CREATE TABLE tabDeviceOnOff (
    DeviceID   INTEGER  NOT NULL,
    OnOff      INTEGER NOT NULL,
    PRIMARY KEY ( DeviceID )
);
CREATE TABLE tabEnablePassiveWorkStationFunctions (
    FunctionID INTEGER
);
CREATE TABLE tabEncryptionKeyList(
    ID INTEGER NOT NULL,
    KeyID INTEGER NOT NULL,
    KeyAlias VARCHAR(32) NOT NULL,
    CKR INTEGER NOT NULL,
    PRIMARY KEY ( ID )
);
CREATE TABLE tabEncryptionSetting (
    ChannelID  INTEGER NOT NULL,
    EncryptionType INTEGER NOT NULL,
    PeriodicKeyfailAlert INTEGER NOT NULL DEFAULT 1,
    ClearAlert INTEGER NOT NULL DEFAULT 0,
    ChkSecureSwitchWhenStrap INTEGER NOT NULL DEFAULT 0,
    DisplayOnSecureSwitch INTEGER NOT NULL DEFAULT 2,
    DisplayOnPTT INTEGER NOT NULL DEFAULT 2,
    DisplayOnChannelChange INTEGER NOT NULL DEFAULT 2,
    PRIMARY KEY(ChannelId,EncryptionType)
);
CREATE TABLE tabFiveToneSignaling(
    SigID  INTEGER NOT NULL,
	Freq0 INTEGER NOT NULL,
    Freq1 INTEGER NOT NULL,
    Freq2 INTEGER NOT NULL,
    Freq3 INTEGER NOT NULL,
    Freq4 INTEGER NOT NULL,
    Freq5 INTEGER NOT NULL,
    Freq6 INTEGER NOT NULL,
    Freq7 INTEGER NOT NULL,
    Freq8 INTEGER NOT NULL,
    Freq9 INTEGER NOT NULL,
    Freq10 INTEGER NOT NULL,
    Freq11 INTEGER NOT NULL,
	Freq12 INTEGER NOT NULL,
    Freq13 INTEGER NOT NULL,
    Freq14 INTEGER NOT NULL,
    Freq15 INTEGER NOT NULL,
    RepeatToneIdx INTEGER NOT NULL,
    GroupToneIdx INTEGER NOT NULL,
    AckToneIdx INTEGER NOT NULL,
    Link2ToneIdx INTEGER NOT NULL,
    SrcIdTxFlag INTEGER NOT NULL,
    Link2TxType INTEGER NOT NULL,
    AckTxType INTEGER NOT NULL,
    RptAccessTxType INTEGER NOT NULL,
    RptIdNumOfDigits INTEGER NOT NULL,
    RptID VARCHAR( 10 ) NOT NULL,
    NormalDigitTxDur INTEGER NOT NULL,
    RepeaterIdTxDur INTEGER NOT NULL,
    FirstSrcIdTxDur INTEGER NOT NULL,
    FirstDestIdTxDur INTEGER NOT NULL,
    LinkrTxDur INTEGER NOT NULL,
    Link1TxDur INTEGER NOT NULL,
    Link2TxDur INTEGER NOT NULL,
    LinkaTxDur INTEGER NOT NULL,
    LeadOutTxDur INTEGER NOT NULL,
    NoSigActTimeout INTEGER NOT NULL,
    ACKWaitTimeout INTEGER NOT NULL,
    RxToneGap INTEGER NOT NULL,
    RxTimeout INTEGER NOT NULL,
    ACKWaitFlag INTEGER NOT NULL,
    PRIMARY KEY ( SigID )
);
CREATE TABLE tabFuncAlertTone (
    FuncID INTEGER,
    Enable INTEGER,
    AlertToneID INTEGER,
    CtrlByDuty INTEGER,
    CtrlByAlertMode INTEGER,
    Alias VARCHAR(30),
    Priority INTEGER,
    PRIMARY KEY ( FuncID )
);
CREATE TABLE tabFuncLEDSettings (
    FuncID INTEGER NOT NULL,
    FuncOnOff INTEGER NOT NULL,
    Priority INTEGER NOT NULL,
    LedID INTEGER NOT NULL,
    ColorID INTEGER NOT NULL,
    Period INTEGER NOT NULL,
    Percent INTEGER NOT NULL,
    Cycle INTEGER NOT NULL,
    Interval INTEGER NOT NULL,
    Count INTEGER NOT NULL,
    PRIMARY KEY ( FuncID )
);
CREATE TABLE tabFuncParam (
    DeviceType INTEGER NOT NULL,
    BandType INTEGER NOT NULL,
    BootUpLogo INTEGER NOT NULL,
    OutRangeCheckTime INTEGER NOT NULL DEFAULT '20',
    InRangeCheckTime INTEGER NOT NULL DEFAULT '5',
    InOutRangeAlert INTEGER NOT NULL DEFAULT '1',
    AlertTimes INTEGER NOT NULL DEFAULT '0',
    AlertInterval INTEGER NOT NULL DEFAULT '180',
    AlertTimeout INTEGER NOT NULL DEFAULT '900',
    ReminderAlert INTEGER NOT NULL DEFAULT '0',
    ReminderAlertInterval INTEGER NOT NULL DEFAULT '180',
    TwoToneAlertType INTEGER NOT NULL DEFAULT '0',
    TwoToneAlertDuration INTEGER NOT NULL DEFAULT '2',
    FiveToneMdcAlertDuration INTEGER NOT NULL DEFAULT '3',
    DigitalAlertDuration INTEGER NOT NULL DEFAULT '2',
    PriorityAlertToneID INTEGER NOT NULL DEFAULT '1',
    RecvAlertVolumeType INTEGER NOT NULL DEFAULT '1',
    RecvAlertVolume INTEGER NOT NULL DEFAULT '5',
    NotificationAlertVolumeType INTEGER NOT NULL DEFAULT '0',
    NotificationAlertVolume INTEGER NOT NULL DEFAULT '3',
    BTHeadsetNotification INTEGER NOT NULL DEFAULT '0',
    TGHoldFunc INTEGER NOT NULL DEFAULT '0',
    TGHoldTimeout INTEGER NOT NULL DEFAULT '30',
    TGRemvFunc INTEGER NOT NULL DEFAULT '0',
    UnholdAlert INTEGER NOT NULL DEFAULT '0',
    DutyStatus INTEGER NOT NULL DEFAULT '1',
    DutyCycle INTEGER NOT NULL DEFAULT '17',
    StartTimeHr INTEGER NOT NULL DEFAULT '9',
    StartTimeMin INTEGER NOT NULL DEFAULT '0',
    EndTimeHr INTEGER NOT NULL DEFAULT '18',
    EndTimeMin INTEGER NOT NULL DEFAULT '0',
    OffDutyRecord INTEGER NOT NULL DEFAULT '1',
    FlashOnTime INTEGER NOT NULL DEFAULT '1',
    FlashOffTime INTEGER NOT NULL DEFAULT '1',
    FlashDuration INTEGER NOT NULL DEFAULT '0',
    IsVoiceFlash INTEGER NOT NULL DEFAULT '0',
    IsTextFlash INTEGER NOT NULL DEFAULT '0',
    IsImageFlash INTEGER NOT NULL DEFAULT '0',
    AutoKeypadLock INTEGER NOT NULL DEFAULT '0',
    KeypadLockComboDetectTime INTEGER NOT NULL DEFAULT '3',
    PlayKeyDepressTime INTEGER NOT NULL DEFAULT '2',
    ShowMsgContent INTEGER NOT NULL DEFAULT '0',
    PlayOption INTEGER NOT NULL DEFAULT '0',
    AlertToneDur INTEGER NOT NULL DEFAULT '15',
    BrightnessMode INTEGER NOT NULL DEFAULT '1',
    BrightnessLevel INTEGER NOT NULL DEFAULT '5',
    LCDSavingTime INTEGER NOT NULL DEFAULT '10',
    SysIDShowType INTEGER NOT NULL DEFAULT '0',
    CallIDShowType INTEGER NOT NULL DEFAULT '0',
    FrequencyUnit INTEGER NOT NULL DEFAULT '0',
    BleName VARCHAR( 13 ) DEFAULT 'Unication_BLE',
    BlePair INTEGER NOT NULL DEFAULT '0',
    UserSettingAdvertiseStatus INTEGER NOT NULL DEFAULT '0',
    BleAdvertiseTime INTEGER NOT NULL DEFAULT '60',
    BleFwVer VARCHAR( 32 ),
    PsdrtIP VARCHAR( 20 ) DEFAULT '203.74.115.194',
    PsdrtPort VARCHAR( 7 ) DEFAULT '8087',
    PsdrtPagerID VARCHAR( 7 ) DEFAULT '0000000',
    DutyStatusACK INTEGER NOT NULL DEFAULT '1',
    OORStatusACK INTEGER NOT NULL DEFAULT '1',
    PowerOffACK INTEGER NOT NULL DEFAULT '1',
    ManualACKKeepTime INTEGER NOT NULL DEFAULT '5',
    WDogOpen INTEGER NOT NULL DEFAULT '1',
    WDogBreed INTEGER NOT NULL DEFAULT '1',
    WDogNormalFeedInterval INTEGER NOT NULL DEFAULT '2',
    WDogPowerSavingFeedInterval INTEGER NOT NULL DEFAULT '240',
    ShutdownTimeAfterSuspend INTEGER NOT NULL DEFAULT '14400',
    DateFormat INTEGER NOT NULL DEFAULT '0',
    HourFormat INTEGER NOT NULL DEFAULT '1',
    DelayCloseSpeakerTimeOfAMP INTEGER NOT NULL DEFAULT '600',
    Pwd1Enable INTEGER NOT NULL DEFAULT '0',
    Pwd1Content VARCHAR( 16 ),
    Pwd2Enable INTEGER NOT NULL DEFAULT '0',
    Pwd2Content VARCHAR( 16 ),
    Alarm1On INTEGER NOT NULL DEFAULT '0',
    Alarm2On INTEGER NOT NULL DEFAULT '0',
    CradleType INTEGER NOT NULL DEFAULT '0',
    VoiceCallMAXTime INTEGER NOT NULL,
    VoiceMemoMAXTime INTEGER NOT NULL,
    SpkOnOffOnceDuringACall INTEGER NOT NULL DEFAULT '1',
    UserCompany VARCHAR( 128 ),
    Password VARCHAR( 20 ),
    StunChID INTEGER NOT NULL DEFAULT '0',
    SignalLevel INTEGER NOT NULL DEFAULT '0'
);
CREATE TABLE tabFuncVibrate (
    FuncID       INTEGER,
    Enable       INTEGER,
    VibPatternID INTEGER,
    VibAlias     VARCHAR( 64 ),
    Type         INTEGER,
    Interval     INTEGER,
    MaxTime      INTEGER,
    VibID        INTEGER,
    PRIMARY KEY ( FuncID )
);
CREATE TABLE tabFunctionSettingList(
  SettingItemSerial INTEGER,
  FuncSettingId INTEGER,
  Enable INTEGER
);
CREATE TABLE tabFunctionStatus(
    SettingItemId INTEGER NOT NULL PRIMARY KEY,
    SettingItemSerial INTEGER NOT NULL,
    Name VARCHAR(64),
    ItemType INTEGER NOT NULL,
    HasChild INTEGER NOT NULL,
    BelongSettingItem INTEGER DEFAULT NULL,
    Status INTEGER
);
CREATE TABLE tabHDPhotoProfile (
    SettingID INTEGER,
    Quantization INTEGER,
    TileWidth INTEGER,
    TileHeight INTEGER,
    PRIMARY KEY ( SettingId )
);
CREATE TABLE tabHardKeyActionOnKeypadLock (
    KeyCode    INTEGER NOT NULL,
    KeyName    VARCHAR,
    ActionType INTEGER NOT NULL
);
CREATE TABLE tabHomAndDetailItem (
    RecvMode    SMALLINT,
    Item        SMALLINT,
    Name        VARCHAR( 30 ),
    Title       VARCHAR( 30 ),
    FuncId      SMALLINT,
    TitleColor1 SMALLINT,
    TitleColor2 SMALLINT
);
CREATE TABLE tabHomeAlarmLayout (
    Style           SMALLINT,
    Type            SMALLINT,
    Name            VARCHAR( 128 ),
    NameColor       VARCHAR( 30 ),
    FontSize        SMALLINT,
    BackgroundColor VARCHAR( 7 ),
    LineHeight      SMALLINT,
    Alignment       VARCHAR( 30 )
);
CREATE TABLE tabHomeScreenItem (
    Style           SMALLINT,
    RecvMode        SMALLINT,
    Item            SMALLINT,
    ItemIndex       SMALLINT,
    ContentColor    VARCHAR( 128 ),
    FontSize        VARCHAR( 30 ),
    BackgroundColor VARCHAR( 7 ),
    LineHeight      SMALLINT,
    Alignment       VARCHAR( 30 )
);
CREATE TABLE tabHomeSettings (
    IsFrequencyVisible  INTEGER  NOT NULL,
    P25TSystemShowOption  INTEGER  NOT NULL,
    P25TSystemName  VARCHAR(20),
    HomeStyle  INTEGER,
    NumberFormat  INTEGER,
    ScanningTime  INTEGER,
    PromptShowTime  INTEGER,
    PromptHideTime  INTEGER,
    ScanFrqTime  INTEGER,
    IsRfssSiteVisible  INTEGER
);
CREATE TABLE tabInformation(
    ID INTEGER NOT NULL,
    Description VARCHAR(200),
    ItemIndex INTEGER NOT NULL,
    Enable INTEGER NOT NULL
);
CREATE TABLE tabKMFSystem (
    KMF_SYS_ID INTEGER NOT NULL DEFAULT 1,
    Name VARCHAR(32) NOT NULL,
    KMF_IP VARCHAR(32) NOT NULL DEFAULT '0.0.0.0',
    KMF_Port INTEGER NOT NULL DEFAULT 64414,
    SU_IP VARCHAR(32) NOT NULL DEFAULT '0.0.0.0',
    SU_Port INTEGER NOT NULL DEFAULT 64414,
    Proxy_BT_MAC  VARCHAR(32)  NOT NULL DEFAULT '00:00:00:00:00:00',
    PRIMARY KEY (KMF_SYS_ID)
);
CREATE TABLE tabKMFSystemByChannel (
    CHID INTEGER NOT NULL,
    ID INTEGER NOT NULL,
    PRIMARY KEY (CHID, ID)
);
CREATE TABLE tabKVLUSBConnect (
    ID INTEGER NOT NULL,
    SU_IP VARCHAR(32) NOT NULL DEFAULT '192.168.128.1',
    SU_Port INTEGER NOT NULL DEFAULT 49165,
    KFD_IP VARCHER(32) NOT NULL DEFAULT '192.168.168.2',
    KFD_Port INTEGER NOT NULL DEFAULT 49165,
    PRIMARY KEY (ID)
);
CREATE TABLE tabKnobInfo (
    ZoneKnobID INTEGER NOT NULL,
    Name VARCHAR(16) NOT NULL,
    RecvMode INTEGER NOT NULL,
    PromptID INTEGER NOT NULL,
    SignalingType INTEGER NOT NULL,
    SigID INTEGER NOT NULL,
    BindingFeature INTEGER NOT NULL,
    ResetMode INTEGER NOT NULL,
    TimeoutResetTimer INTEGER NOT NULL,
    TGIDStayFunction INTEGER NOT NULL,
    TGIDStayTimer INTEGER NOT NULL,
    DelayNFunction INTEGER NOT NULL,
    DelayNTimer INTEGER NOT NULL,
    RecDelayFunction INTEGER NOT NULL,
    RecDelayTimer INTEGER NOT NULL,
    PTL INTEGER NOT NULL,
    Privacy INTEGER NOT NULL,
    MonitorModeOn INTEGER NOT NULL,
    VoiceStorage INTEGER NOT NULL,
    VoiceBuffering INTEGER NOT NULL,
    VoiceBufferingPrompt INTEGER NOT NULL,
    AlertMode INTEGER NOT NULL,
    UnreadMsgOpt INTEGER NOT NULL,
    UnreadMsgAlert INTEGER NOT NULL,
    SquelchOption INTEGER NOT NULL,
    SecureFunction INTEGER NOT NULL,
	ForceDrop INTEGER NOT NULL,
    PRIMARY KEY ( ZoneKnobID )
);
CREATE TABLE tabMDC1200Signaling (
    SigID INTEGER NOT NULL,
    BasicTxPreTime INTEGER NOT NULL DEFAULT '1',
    RptAccTxPreTime INTEGER NOT NULL DEFAULT '1',
    PmbBitSync INTEGER NOT NULL DEFAULT '24',
    PTTIDType INTEGER NOT NULL DEFAULT '0',
    AutoRptTxType INTEGER NOT NULL DEFAULT '0',
    TxRptID INTEGER NOT NULL DEFAULT '0',
    InbnMsgRxEnable INTEGER NOT NULL DEFAULT '0',
    SignallingActivityTimeout INTEGER NOT NULL DEFAULT '0',
    ACKWaitTimeout INTEGER NOT NULL,
    UniClass1Msg INTEGER NOT NULL,
    UniClass2Msg INTEGER NOT NULL,
    UniClass3Msg INTEGER NOT NULL,
    PRIMARY KEY ( SigID )
);
CREATE TABLE tabManualACKContent (
    MessageID INTEGER,
    MessageContent VARCHAR( 250 ),
    PRIMARY KEY ( MessageID )
);
CREATE TABLE tabMemberList (
    ID INTEGER NOT NULL,
    SignalingType INTEGER NOT NULL,
    Address VARCHAR (30) NOT NULL,
    Alias VARCHAR (14) NOT NULL,
	SigID INTEGER NOT NULL,
    PRIMARY KEY (ID,SignalingType)
);
CREATE TABLE tabMemory (
    FunID INTEGER PRIMARY KEY CHECK ( funId > 0 ),
    MemType INTEGER NOT NULL CHECK ( memType BETWEEN 0 AND 1 ) DEFAULT '0',
    MaxMem INTEGER NOT NULL CHECK ( maxMem > 0 ),
    OverflowType INTEGER NOT NULL CHECK ( overflowType BETWEEN 0 AND 1 ) DEFAULT '1',
    HasAlert INTEGER NOT NULL CHECK ( hasAlert BETWEEN 0 AND 1 ) DEFAULT '0',
    AlertLv1 INTEGER NOT NULL CHECK ( alertLv1 > 0 ),
    AlertLv2 INTEGER NOT NULL CHECK ( alertLv2 > 0 ),
    Alias CHAR( 64 )
);
CREATE TABLE tabMenuListLayout (
    Position INTEGER NOT NULL,
    ItemKey INTEGER NOT NULL
);
CREATE TABLE tabP25OTARFeatureControl (
    ID INTEGER NOT NULL,
    P25OTAREnable INTEGER NOT NULL DEFAULT 0,
    P25OTARRadioID INTEGER NOT NULL DEFAULT 1,
    P25OTARRxSecurityLevel INTEGER NOT NULL DEFAULT 0,
    P25OTARTxSecurityLevel INTEGER NOT NULL DEFAULT 0,
    ResponseKind INTEGER NOT NULL DEFAULT 0,
    TimeBetweenAttempts INTEGER NOT NULL DEFAULT 15,
    UserSelectableRekeyRequest INTEGER NOT NULL DEFAULT 0,
    InactivityTime INTEGER NOT NULL DEFAULT 72,
    RadioInhibit INTEGER NOT NULL DEFAULT 1,
    UserSelectableKeyset INTEGER NOT NULL DEFAULT 0,
    UserSelectableKey INTEGER NOT NULL DEFAULT 0,
    NetworkAddrType INTEGER NOT NULL DEFAULT 0,
    SU_IP VARCHAR(32) NOT NULL DEFAULT '0.0.0.0',
    SU_Port INTEGER NOT NULL DEFAULT 64414,
    PacketDataRegVersion INTEGER NOT NULL DEFAULT 3,
    IPHeaderCompression  INTEGER NOT NULL DEFAULT 0,
    DAC INTEGER NOT NULL DEFAULT 1,
    primary key (ID)
);
CREATE TABLE tabP25OTARFeatureControlByChannel (
    CHID INTEGER NOT NULL,
    ID INTEGER NOT NULL,
    PRIMARY KEY (CHID, ID)
);
CREATE TABLE tabP25Signaling (
    SigID INTEGER NOT NULL,
    TXPreTime INTEGER NOT NULL,
    NoSigActTimeout INTEGER NOT NULL,
    ACKWaitTimeout INTEGER NOT NULL,
    RxNAC INTEGER NOT NULL,
    MFID INTEGER NOT NULL,
    AlgId INTEGER NOT NULL,
    MinShortDigitalToneDuration INTEGER NOT NULL,
    MinLongDigitalToneDuration INTEGER NOT NULL,
    MinToneAddrGap INTEGER NOT NULL,
    ShortDigitalToneATxDuration INTEGER NOT NULL,
    ShortDigitalToneBTxDuration INTEGER NOT NULL,
    LongDigitalToneTxDuration INTEGER NOT NULL,
    TwoDigitalToneAddrTxGap INTEGER NOT NULL,
    EncryptedCallFlag INTEGER NOT NULL,
    PRIMARY KEY ( SigID )
);
CREATE TABLE tabP25TSetting
(
    ChannelID INTEGER NOT NULL,
    ATGID INTEGER NOT NULL,
    EncryptedCallFlag INTEGER NOT NULL,
    MSELCallSamplingFlag INTEGER NOT NULL,
    MaxMSELSamplingTime INTEGER NOT NULL,
    PRIMARY KEY(ChannelID)
);
CREATE TABLE tabP25TrunkingSignaling (
    SigID INTEGER NOT NULL,
    PhaseIIRx INTEGER NOT NULL,
    RfssRspWaitTimeout INTEGER NOT NULL,
    MFID INTEGER DEFAULT "0",
    AlgId INTEGER NOT NULL,
    SystemID INTEGER NOT NULL,
    WacnID INTEGER NOT NULL,
    SiteTrunkingFlag INTEGER NOT NULL,
    RssiThreshold INTEGER NOT NULL,
    FullSpectrumScanRanges INTEGER NOT NULL,
    StartFreqOfRange1 INTEGER NOT NULL,
    EndFreqOfRange1 INTEGER NOT NULL,
    StepSizeOfRange1 INTEGER,
    StartFreqOfRange2 INTEGER,
    EndFreqOfRange2 INTEGER,
    StepSizeOfRange2 INTEGER,
    StartFreqOfRange3 INTEGER,
    EndFreqOfRange3 INTEGER,
    StepSizeOfRange3 INTEGER,
    MinShortDigitalToneDuration INTEGER NOT NULL,
    MinLongDigitalToneDuration INTEGER NOT NULL,
    MinToneAddrGap INTEGER NOT NULL,
    ShortDigitalToneATxDuration INTEGER NOT NULL,
    ShortDigitalToneBTxDuration INTEGER NOT NULL,
    LongDigitalToneTxDuration INTEGER NOT NULL,
    TwoDigitalToneAddrTxGap INTEGER NOT NULL,
    SystemName VARCHAR (14) NOT NULL,
    SUID       INTEGER,
    PRIMARY KEY ( SigID )
);
CREATE TABLE tabP25TrunkingSites (
    SigID INTEGER NOT NULL,
    ItemType INTEGER NOT NULL,
    RfssID INTEGER NOT NULL,
    SiteID INTEGER NOT NULL,
    RxChFreq INTEGER NOT NULL,
    SiteName VARCHAR( 14 ) NOT NULL DEFAULT "",
    SeqNo INTEGER NOT NULL DEFAULT "0"
);
CREATE TABLE tabPagerMsgBoxCapacity (
    Type INTEGER,
    MaxNum INTEGER,
    MaxLockNum INTEGER,
    PRIMARY KEY ( Type )
);
CREATE TABLE tabRecvBehaviorByContactID (
    ZoneKnobID INTEGER NOT NULL,
    ContactID INTEGER NOT NULL,
	SigType INTEGER  NOT NULL,
    Status INTEGER NOT NULL,
    VoiceRecvAck INTEGER NOT NULL,
    VoiceReadAck INTEGER NOT NULL,
    VoiceManualAck INTEGER NOT NULL,
    TextRecvAck INTEGER NOT NULL,
    TextReadAck INTEGER NOT NULL,
    TextManualAck INTEGER NOT NULL,
    ImageRecvAck INTEGER NOT NULL,
    ImageReadAck INTEGER NOT NULL,
    ImageManualAck INTEGER NOT NULL,
    PRIMARY KEY ( ZoneKnobID, ContactID, SigType )
);
CREATE TABLE tabRecvGroupList (
    ZoneKnobID INTEGER NOT NULL,
    SigType INTEGER NOT NULL,
    SigID INTEGER NOT NULL,
    ContactID INTEGER,
    SeqNo INTEGER,
    PRIMARY KEY ( ZoneKnobID, SigType, SigID, ContactID )
);
CREATE TABLE tabRecvImageForShowSetting (
    Good INTEGER NOT NULL,
    Normal INTEGER NOT NULL,
    Poor INTEGER NOT NULL
);
CREATE TABLE tabRevertListGroup (
    ZoneKnobID INTEGER NOT NULL,
    RevertAddress VARCHAR( 30 ) NOT NULL,
    SeqNo INTEGER NOT NULL
);
CREATE TABLE tabRfUtility (
    RssiNormal  INTEGER NOT NULL,
    RssiLow  INTEGER NOT NULL,
    RssiWeak  INTEGER NOT NULL,
    RssiLost  INTEGER NOT NULL,
    SquelchLevel  INTEGER NOT NULL,
    SQLow  INTEGER NOT NULL,
    SQMedium  INTEGER NOT NULL,
    SQHigh  INTEGER NOT NULL
);
CREATE TABLE tabScanListSettings (
    ScanListSettingID INTEGER NOT NULL,
    PriorityScanType INTEGER NOT NULL,
    SamplingFlag INTEGER NOT NULL,
    PriorityZoneKnobID INTEGER NOT NULL,
    ScanList VARCHAR( 1024 ) NOT NULL,
    RecvAllFlag INTEGER NOT NULL,
    StayTime INTEGER NOT NULL,
    CallBackTime INTEGER NOT NULL,
    RxPauseBetweenScanPaths INTEGER NOT NULL DEFAULT 0,
    PriorityIntervalTime INTEGER NOT NULL DEFAULT 5,
	CtCdRequest INTEGER NOT NULL,
    PRIMARY KEY ( ScanListSettingID )
);
CREATE TABLE tabSpecialKeyActionTime(
    KeyCode    INTEGER NOT NULL,
    KeyName    VARCHAR,
    ActionTime INTEGER NOT NULL,
    primary key (KeyCode)
);
CREATE TABLE tabTextMessageSetting (
    SettingID INTEGER,
    MaxTextNoteLen INTEGER,
    MergeMaxDuration INTEGER,
    SimilarPercent INTEGER,
    PRIMARY KEY ( SettingID )
);
CREATE TABLE tabTwoToneSignaling (
    SigID INTEGER NOT NULL,
    PreTime INTEGER DEFAULT 10 NOT NULL,
    Tone1TxDuration INTEGER DEFAULT 200 NOT NULL,
    Gap1TxDuration INTEGER DEFAULT 0 NOT NULL,
    Tone2TxDuration INTEGER DEFAULT 200 NOT NULL,
    Gap2TxDuration INTEGER DEFAULT 0 NOT NULL,
    LongToneDuration INTEGER DEFAULT 200 NOT NULL,
    LongToneTxGap INTEGER DEFAULT 0 NOT NULL,
    LongToneRxTmout INTEGER DEFAULT 3000 NOT NULL,
    SignallingActivityTimeout INTEGER DEFAULT 10 NOT NULL,
    RxTimeout INTEGER DEFAULT 3000 NOT NULL,
    OtaTonePlayFlag INTEGER DEFAULT 1 NOT NULL,
    MinToneDuration INTEGER DEFAULT 400 NOT NULL,
    NSigPwrOnThreshold INTEGER default 3162278 NOT NULL,
    NSigPwrOffThreshold INTEGER default 1778279 NOT NULL,
    WSigPwrOnThreshold INTEGER default 3162278 NOT NULL,
    WSigPwrOffThreshold INTEGER default 3162278 NOT NULL,
    PRIMARY KEY ( SigID )
);
CREATE TABLE tabUseDefineCHDetail (
    ZoneKnobID INTEGER,
    DateTime VARCHAR (32),
    SignalingType INTEGER,
    RXFrequency INTEGER,
    RXBandWidth INTEGER,
    FromAddr VARCHAR (32),
    ToAddr VARCHAR (32),
    NAC INTEGER,
    ColorCode INTEGER,
    SubAudio VARCHAR (64),
    BER INTEGER,
    Encryption INTEGER,
    EmgService INTEGER,
    Priority INTEGER,
    SignalLevel INTEGER
);
CREATE TABLE tabVibrateDetail(
    VibID    INTEGER NOT NULL,
    IndexID  INTEGER NOT NULL,
    Enable   INTEGER NOT NULL,
    Duration INTEGER NOT NULL,
    PRIMARY KEY ( VibID, IndexID )
);
CREATE TABLE tabZone (
    ZoneID INTEGER NOT NULL,
    ZoneName VARCHAR(30) NOT NULL,
    PRIMARY KEY ( ZoneID )
);
