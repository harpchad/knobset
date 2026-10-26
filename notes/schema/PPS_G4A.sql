CREATE TABLE tabAccessPassword(
    ID              INTEGER NOT NULL,
    Enable          INTEGER,
    AccessPassword  VARCHAR(64),
    primary key (ID)
);
CREATE TABLE tabAdvancedChannelGroup (
    ChannelID  VARCHAR(64) NOT NULL,
    GroupID  VARCHAR(64) NOT NULL,
    SeqNo INTEGER NOT NULL,
    ProtocolSettingID VARCHAR(64) NOT NULL,
    OffDuty INTEGER,
    Priority INTEGER,
    OutOfRange INTEGER,
    OTA INTEGER,
    VoiceRecvAck INTEGER,
    VoiceReadAck INTEGER,
    VoiceManualAck INTEGER,
    TextRecvAck INTEGER,
    TextReadAck INTEGER,
    TextManualAck INTEGER,
    ImageRecvAck INTEGER,
    ImageReadAck INTEGER,
    ImageManualAck INTEGER
);
CREATE TABLE tabAdvancedConvCHKnob(
    ZoneID VARCHAR (64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    ChannelID VARCHAR (64) NOT NULL,
    ConvFreqID VARCHAR (64) NOT NULL,
    CSQProtocolSettingID VARCHAR(64),
    ToneProtocolSettingID VARCHAR(64),
    MDCProtocolSettingID VARCHAR(64),
    DigitalProtocolSettingID VARCHAR(64),
    ReserveProtocolSettingID VARCHAR(64),
    RxSaType INTEGER NOT NULL DEFAULT 0,
    RxCTFreq INTEGER NOT NULL DEFAULT 0,
    RxCDCodeWord INTEGER NOT NULL DEFAULT 0,
    RxCDModSense INTEGER NOT NULL DEFAULT 0,
    TxSaType INTEGER NOT NULL DEFAULT 0,
    TxCTFreq INTEGER NOT NULL DEFAULT 0,
    TxCDCodeWord INTEGER NOT NULL DEFAULT 0,
    TxCDModSense INTEGER NOT NULL DEFAULT 0,
    RxNAC INTEGER NOT NULL,
    TxNAC INTEGER NOT NULL,
    RxColorCode INTEGER NOT NULL,
    TxColorCode INTEGER NOT NULL,
    Emphasis INTEGER NOT NULL,
    Compander INTEGER NOT NULL,
    TxPowerLevel INTEGER NOT NULL,
    AudioSquelch INTEGER NOT NULL,
    OpenSpeakerLogic INTEGER NOT NULL,
    HangTime INTEGER NOT NULL,
    DefaultGroupID VARCHAR (64),
    DispatchCHFlag INTEGER NOT NULL DEFAULT '0',
    Encryption INTEGER NOT NULL DEFAULT '0',
    RFPerformance INTEGER NOT NULL DEFAULT '1',
    PushToListen INTEGER NOT NULL DEFAULT '0',
    Privacy INTEGER NOT NULL DEFAULT '0',
    SquelchOption INTEGER NOT NULL,
    VoiceBuffering INTEGER NOT NULL,
	VoiceBufferPrompt INTEGER NOT NULL DEFAULT '0',
    VoiceBufferingForceDrop INTEGER NOT NULL DEFAULT 0,
    ResetMode INTEGER DEFAULT '0',
    DelayNTime INTEGER DEFAULT '30',
    EnableCarrierOffTime INTEGER DEFAULT '0',
    CarrierOffDelayTime INTEGER DEFAULT '30',
    AlertMode INTEGER,
    RecordFlag INTEGER,
    DutyStatus INTEGER,
    TransmitMode INTEGER NOT NULL,
	AlwaysUnreadVoiceMessage INTEGER NOT NULL,
	AlwaysUnreadReminderAlert INTEGER NOT NULL,
	EncryptedCallFlag INTEGER NOT NULL DEFAULT 0,
    EncryptionBy INTEGER DEFAULT 0 NOT NULL,
    EncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    KeyStrapOption INTEGER NOT NULL DEFAULT 1,
    KMFSystemID VARCHAR(64) NOT NULL DEFAULT '000000000000',
    PrivateEncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    PrivateKeyStrapOption INTEGER NOT NULL DEFAULT 1,
    UserSelectableKeyset INTEGER NOT NULL DEFAULT 0,
	UserSelectableKey INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (ZoneID, KnobIndex)
);
CREATE TABLE tabAllBandScan(
    ZoneID  VARCHAR(64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    StartFreq INTEGER NOT NULL,
    EndFreq INTEGER NOT NULL,
    StepSize INTEGER NOT NULL,
    StayTime INTEGER NOT NULL,
    OpenSpeaker INTEGER NOT NULL,
    Primary key(ZoneID,KnobIndex)
);
CREATE TABLE tabAllBandScanBandParam(
    ChannelID VARCHAR(64) NOT NULL,
    SeqNo INTEGER NOT NULL,
    RFBandTypeID INTEGER NOT NULL,
    StartFreq INTEGER NOT NULL,
    EndFreq INTEGER NOT NULL,
    StepSize INTEGER NOT NULL,
    Primary key(ChannelID,SeqNo)
);
CREATE TABLE tabAllBandScanCHKnob(
    ZoneID  VARCHAR(64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    ChannelID VARCHAR(64) NOT NULL,
    BandWide INTEGER NOT NULL,
    StayTime INTEGER NOT NULL,
    RxPauseBetweenScanPaths INTEGER NOT NULL DEFAULT 0,
    AutoContinueTime INTEGER NOT NULL DEFAULT 0,
    AlertMode INTEGER,
    RecordFlag INTEGER,
    DutyStatus INTEGER,
    Primary key(ZoneID,KnobIndex)
);
CREATE TABLE tabAllBandScanProtocolParam(
    ChannelID VARCHAR(64) NOT NULL,
    ProtocolType INTEGER NOT NULL,
    OnOff INTEGER NOT NULL,
    Vocoder INTEGER NOT NULL,
    Primary key(ChannelID,ProtocolType)
);
CREATE TABLE tabArtilleryParam (ID VARCHAR(64), Name VARCHAR(64), InitialVelocity INTEGER, MinAttackRange INTEGER, MaxAttackRange INTEGER, MinElevationAngle FLOAT, MaxElevationAngle FLOAT);
CREATE TABLE tabAudioToneSetting(
   ToneType INTEGER NOT NULL,
   ToneVolume INTEGER NOT NULL
);
CREATE TABLE [tabBacklightModeSettings] ([ID] INTEGER NOT NULL, [Mode] INTEGER NOT NULL DEFAULT '0', [Brightness] SMALLINT NOT NULL DEFAULT '0', [BrightLevel] INTEGER, PRIMARY KEY ([ID]));
CREATE TABLE tabBacklightSettings(
    id  INTEGER NOT NULL,
    DefaultTime INTEGER default 10 NOT NULL,
    ModeInGPSDisplay INTEGER default 0 NOT NULL,
    ModeInScan  INTEGER default 0 NOT NULL,
    ModeInEditTextMsg INTEGER default 0 NOT NULL,
    TimeInGPSDisplay INTEGER default 10 NOT NULL,
    TimeInScan INTEGER default 10 NOT NULL,
    TimeInEditTextMsg  INTEGER default 10 NOT NULL,
    primary key (id)
);
CREATE TABLE tabBleParamSetting (
    BlePair                    INTEGER        NOT NULL,
    UserSettingAdvertiseStatus INTEGER        NOT NULL,
    BleFwVer                   VARCHAR(32),
    BleName                    VARCHAR(13),
    BleAdvertiseTime           INTEGER        NOT NULL
);
CREATE TABLE tabBtHeadsetSetting (
    BTHeadsetNotification SMALLINT NOT NULL DEFAULT '0'
);
CREATE TABLE tabBusinessOption (
    ChannelLimit INT,
    ZoneLimit INT
);
CREATE TABLE tabCDCSSSignaling(
    ID  VARCHAR(64) NOT NULL,
    ModSense INTEGER NOT NULL,
    Ste INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabCTCSSSignaling(
    ID  VARCHAR(64) NOT NULL,
    Ste INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabCapturePhotoSettings(
    CameraId INTEGER,
    OpticalZoom INTEGER,
    ShuttleMode INTEGER,
    ExposeTriger INTEGER,
    ISO INTEGER,
    Aperture INTEGER,
    ResolutionX INTEGER,
    ResolutionY INTEGER,
    DigitalZoom INTEGER,
    WhiteBalance INTEGER,
    Saturation INTEGER,
    Contract INTEGER,
    Denoise INTEGER,
    Sharpness INTEGER,
    ColorExpress INTEGER,
    BitFormat INTEGER,
    FrameRate INTEGER,
    Brightness INTEGER,
    PRIMARY KEY(CameraId)
);
CREATE TABLE tabChannel(
    ID  VARCHAR(64) NOT NULL,
    TransmitModeID VARCHAR(64) NOT NULL,
    Name VARCHAR(14) NOT NULL,
    RxFreq INTEGER NOT NULL,
    TxFreq INTEGER NOT NULL,
    RxBandWide INTEGER NOT NULL,
    TxBandWide INTEGER NOT NULL,
    TxPowerLever INTEGER NOT NULL,
    AudioSquelch INTEGER NOT NULL,
    OpenSpeakerLogic INTEGER NOT NULL,
    EnableModifyPower INTEGER NOT NULL,
    TxPowerMain INTEGER NOT NULL,
    TxPowerTrans INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabChannelAdvInfo(
    ChannelID  VARCHAR(64) NOT NULL,
    DefaultPTTGroupID INTEGER NOT NULL,
    FirstProtocol INTEGER NOT NULL,
    FirstSaType INTEGER,
    SecondProtocol INTEGER,
    SecondSaType INTEGER,
    FirstRxCode INTEGER,
    FirstTxCode INTEGER,
    SecondRxCode INTEGER,
    SecondTxCode INTEGER,
    PRIMARY KEY(ChannelID)
);
CREATE TABLE tabChannelGroup(
    ChannelID VARCHAR (64) NOT NULL,
    GroupID VARCHAR (64) NOT NULL,
    SigType INTEGER  NOT NULL,
    SeqNo INTEGER NOT NULL,
    OffDuty INTEGER,
    Priority INTEGER,
    OutOfRange INTEGER,
    OTA INTEGER,
    VoiceRecvAck INTEGER,
    VoiceReadAck INTEGER,
    VoiceManualAck INTEGER,
    TextRecvAck INTEGER,
    TextReadAck INTEGER,
    TextManualAck INTEGER,
    ImageRecvAck INTEGER,
    ImageReadAck INTEGER,
    ImageManualAck INTEGER
);
CREATE TABLE tabChannelGroupList(
    ChannelID VARCHAR (64) NOT NULL,
    GroupingID VARCHAR (64) NOT NULL,
    isRevert INTEGER
);
CREATE TABLE tabChannelRevertGroup(
    ChannelID VARCHAR (64) NOT NULL,
    GroupID VARCHAR (64) NOT NULL,
    SeqNo INTEGER NOT NULL,
    OffDuty INTEGER,
    Priority INTEGER,
    OutOfRange INTEGER,
    OTA INTEGER,
    VoiceRecvAck INTEGER,
    VoiceReadAck INTEGER,
    VoiceManualAck INTEGER,
    TextRecvAck INTEGER,
    TextReadAck INTEGER,
    TextManualAck INTEGER,
    ImageRecvAck INTEGER,
    ImageReadAck INTEGER,
    ImageManualAck INTEGER
);
CREATE TABLE tabConnedMessage(
    ID VARCHAR(64) NOT NULL,
    Content VARCHAR(128) NOT NULL
);
CREATE TABLE tabConsole(
    ID VARCHAR(64) NOT NULL,
    ConsoleNo VARCHAR(32) NOT NULL,
    Name VARCHAR(14) NOT NULL,
    NOTE VARCHAR(250),
    PRIMARY KEY(ID)
);
CREATE TABLE tabConsoleAddress(
    ConsoleID  VARCHAR(64) NOT NULL,
    Protocoltype INTEGER NOT NULL,
    Address VARCHAR(30),
    PRIMARY KEY(ConsoleID,Protocoltype)
);
CREATE TABLE tabConvFreqList
(
  FreqID VARCHAR(64) NOT NULL,
  Name VARCHAR(14) NOT NULL,
  TransmitMode INTEGER NOT NULL,
  RxFreq INTEGER NOT NULL,
  TxFreq INTEGER NOT NULL,
  RxBandWidth INTEGER NOT NULL,
  TxBandWidth INTEGER NOT NULL,
  Primary key(FreqID )
);
CREATE TABLE tabCustomAlertTone(
    ID INTEGER PRIMARY KEY,
    ToneAlias VARCHAR (32),
    SourceFileName VARCHAR (256),
    ToneFileName VARCHAR (32)
);
CREATE TABLE tabCustomer (
    CustomerNationCode VARCHAR(2),
    CustomerType VARCHAR(2),
    CustomerCode VARCHAR(2),
    CustomerName VARCHAR(32),
    CustomerNote VARCHAR(240),
    Language VARCHAR(16),
    Version INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE [tabCustomizedFunctionList] ([MemberID] VARCHAR (64), [FunctionID] INTEGER, [Enable] INTEGER);
CREATE TABLE [tabCustomizedFunctionMember] ([MemberID] VARCHAR (64));
CREATE TABLE tabDBVersion (
    MainVersion VARCHAR(16) NOT NULL,
    SubVersion VARCHAR(16) NOT NULL,
    Comment VARCHAR(1024)
);
CREATE TABLE tabDMRRampCodeParam(
    TransmitModeID VARCHAR(64) NOT NULL,
    RxColorCode SMALLINT NOT NULL Default 0,
    TxColorCode SMALLINT NOT NULL Default 0,

    FOREIGN KEY(TransmitModeID) REFERENCES tabTransmitMode(TransmitModeID)
);
CREATE TABLE tabDMRRepeaterSetting(
    ID  VARCHAR(64) NOT NULL,
    TimeSlot INTEGER NOT NULL,
    TimingAlign INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabDMRSignaling(
    ID  VARCHAR(64) NOT NULL,
    TXPreFrames INTEGER NOT NULL,
    RCRxEnable INTEGER NOT NULL,
    NoSigActTimeout INTEGER NOT NULL,
    ACKWaitTimeout INTEGER NOT NULL,
    MFID INTEGER NOT NULL,
    SAP4NRS INTEGER NOT NULL,
    SAP4RS INTEGER NOT NULL,
    DMRCompatibility INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(ID)
);
CREATE TABLE [tabDSPEngineerParam] ([ID] INTEGER, [Value] INTEGER);
CREATE TABLE [tabDSPEngineerSetting] ([Enable] INTEGER);
CREATE TABLE tabDTMFActivationInfo(
    TransmitModeID VARCHAR(64) NOT NULL,
    code1 SMALLINT NOT NULL Default 0,
    code2 SMALLINT NOT NULL Default 0,
    code3 SMALLINT NOT NULL Default 0,
    code4 SMALLINT NOT NULL Default 0,
    ToneMinimumDuration SMALLINT NOT NULL Default 5
);
CREATE TABLE tabDateTimeSetting (
    date_format INT NOT NULL,
    hour_format INT NOT NULL
);
CREATE TABLE tabDebugEntry(
    ID INTEGER NOT NULL,
    Name VARCHAR(64) NOT NULL,
    Enable INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tabDefaultZone (
    DefaultZone VARCHAR(64) NOT NULL
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
CREATE TABLE tabDeviceMember (RadioNo VARCHAR(32), ID VARCHAR(64), DevTypeId INTEGER, MemberID INTEGER, SerialNo Varchar(32));
CREATE TABLE tabDeviceOnOff(
    DeviceID INTEGER NOT NULL,
    OnOff INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tabDeviceType (DevTypeID VARCHAR(64), DevBrand INTEGER, DevModel INTEGER, DevNumber INTEGER);
CREATE TABLE tabDispathCenterACK (
    ManualACKKeepTime INTEGER
);
CREATE TABLE tabDispathCenterManualAck(
    id VARCHAR(64),
    text VARCHAR(512),
    PRIMARY KEY(id)
);
CREATE TABLE tabDutyStatusSetting(
    DefaultDutyStatus INTEGER DEFAULT 1,
    OffDutyRecordFlag INTEGER DEFAULT 1,
    DutyCycle INTEGER,
    StartTimeHr INTEGER,
    StartTimeMin INTEGER,
    EndTimeHr INTEGER,
    EndTimeMin INTEGER
);
CREATE TABLE tabEmergencySetting(
   SettingID  VARCHAR(64),
   EmgType  INTEGER,
   ChannelID VARCHAR(64),
   ContactType INTEGER,
   ContactID  VARCHAR(64),
   ACKTimeLimit INTEGER,
   PoliteRetryTimes INTEGER,
   ImpoliteRetryTimes INTEGER,
   HotMicLength  INTEGER,
   PromptFileName VARCHAR(64),
   GPSInformInterval INTEGER,
   PRIMARY KEY(SettingID)
);
CREATE TABLE tabEncryptionKeyList(
    ID VARCHAR(64) NOT NULL,
    KeyID INTEGER NOT NULL,
    KeyAlias VARCHAR(32) NOT NULL,
    CKR INTEGER NOT NULL,
    primary key (ID)
);
CREATE TABLE tabEncryptionSetting(
    ID VARCHAR(64) NOT NULL,
    EncryptionType INTEGER NOT NULL,
    DefaultSecureSwitch INTEGER NOT NULL DEFAULT 1,
    PeriodicKeyfailAlert INTEGER NOT NULL DEFAULT 1,
    ClearAlert INTEGER NOT NULL DEFAULT 0,
    ChkSecureSwitchWhenStrap INTEGER NOT NULL DEFAULT 0,
    DisplayOnSecureSwitch INTEGER NOT NULL DEFAULT 1,
    DisplayOnPTT INTEGER NOT NULL DEFAULT 1,
    DisplayOnChannelChange INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY(ID)
);
CREATE TABLE tabExternalHWSetting(
    ExternalSpkMicPtt INTEGER NOT NULL,
    BTSpkMicPtt INTEGER NOT NULL,
    BTVideoRecorder INTEGER NOT NULL,
    MiniCell INTEGER NOT NULL,
    EncryptionModule INTEGER NOT NULL
);
CREATE TABLE tabExternalSWSetting(
   MapMaker INTEGER NOT NULL,
   PoliceDBMaker INTEGER NOT NULL,
   EncryptionKeyGenerator INTEGER NOT NULL
);
CREATE TABLE tabExternalSystem(
   MCPE INTEGER NOT NULL
);
CREATE TABLE tabFiveToneFreqSets(
    ID  VARCHAR(64) NOT NULL,
    Name VARCHAR(32),
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
    PRIMARY KEY(ID)
);
CREATE TABLE tabFiveToneRepeaterSetting(
    ID  VARCHAR(64) NOT NULL,
    DigitNum SMALLINT  NOT NULL,
    ToneDigit1 SMALLINT,
    ToneDigit2 SMALLINT,
    ToneDigit3 SMALLINT,
    ToneDigit4 SMALLINT,
    ToneDigit5 SMALLINT,
    ToneDigit6 SMALLINT,
    ToneDigit7 SMALLINT,
    RepeaterAccessTxType INTEGER NOT NULL,
    FirstRepeaterIdTxDuration INTEGER NOT NULL,
    Tonelink1TxDuration INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabFiveToneSignaling(
    ID VARCHAR(64),
    FreqSetID VARCHAR(64),
    SrcIdTxFlag INTEGER,
    Link2TxType INTEGER,
    AckTxType INTEGER,
    NormalDigitTxDur INTEGER,
    FirstSrcIdTxDur INTEGER,
    FirstDestIdTxDur INTEGER,
    LinkrTxDur INTEGER,
    Link2TxDur INTEGER,
    LinkaTxDur INTEGER,
    LeadOutTxDur INTEGER,
    NoSigActTimeout INTEGER,
    ACKWaitTimeout INTEGER,
    RxToneGap INTEGER,
    RxTimeout INTEGER,
    ACKWaitFlag INTEGER
);
CREATE TABLE [tabFixAlertToneSetting] ([Enable] INTEGER, [ToneVolumn] INTEGER);
create table tabFuncClassToneVolume(
    funcClass smallint,
    type smallint,
    volume smallint,
    primary key (funcClass)
);
CREATE TABLE tabFuncLEDSettings (
    funId    INTEGER,
    period   INTEGER,
    percent  INTEGER,
    cycle    INTEGER,
    interval INTEGER,
    count    INTEGER,
    PRIMARY KEY ( funId )
);
CREATE TABLE tabFunctionAlertTone(
    ZoneID  VARCHAR(64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    FuncID INTEGER NOT NULL DEFAULT 0,
    AlertToneID INTEGER,
    AlertFileName VARCHAR(25),
    PRIMARY KEY(FuncID)
);
CREATE TABLE tabFunctionList(
  FunctionID INTEGER,
  Enable INTEGER
);
CREATE TABLE [tabGPRSAckSettingPPS](
    [FuncOption] INTEGER NOT NULL DEFAULT '0',
	[IP] CHAR (40) NOT NULL DEFAULT '203.74.115.194',
	[Port] CHAR (5) NOT NULL DEFAULT '8087',
	[ServerAPN] CHAR (32) NOT NULL DEFAULT 'INTERNET',
	[NameAPN] CHAR (32) NOT NULL DEFAULT 'IP',
	[UserNameAPN] CHAR (32),
	[PasswordAPN] CHAR (32),
    [DutyStatusACK] INTEGER NOT NULL DEFAULT '0',
    [InOutRangeACK] INTEGER NOT NULL DEFAULT '0',
    [PowerOffACK]   INTEGER NOT NULL DEFAULT '0'
);
CREATE TABLE [tabGPRSReceiveSettingPPS] ([FuncOption] INTEGER NOT NULL DEFAULT '0', [IP1] CHAR (40) NOT NULL DEFAULT '59.124.64.98', [Port1] CHAR (5) NOT NULL DEFAULT '9103', [IP2] CHAR (40) NOT NULL DEFAULT '59.124.64.99', [Port2] BOOLEAN (5) NOT NULL DEFAULT '9103', [ConnectMsgOOR] CHAR (40) NOT NULL DEFAULT 'GPRS Connect', [DisconnectMsgOOR] CHAR (40) NOT NULL DEFAULT 'GPRS Disconnect', [SignalCheckTime] INTEGER NOT NULL DEFAULT '3', [AliveMsgInterval] INTEGER NOT NULL DEFAULT '30');
CREATE TABLE tabGPSDisplaySetting(GPSShowType INTEGER NOT NULL DEFAULT 0);
CREATE TABLE tabGPSEngineSetting(
    SaveGPSInterval INTEGER NOT NULL,
    SaveGPSDistance INTEGER NOT NULL DEFAULT 0,
    LostMonitor INTEGER NOT NULL DEFAULT 0,
    LostTime INTEGER NOT NULL DEFAULT 360,
    MonitorInterval INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE tabGPSInform (IntervalForConsole INTEGER, ConsoleID VARCHAR(64), ID VARCHAR(64), Name VARCHAR(14), ConsoleOnOff INTEGER, ConsoleCHID VARCHAR(64), TempConsoleOnOff INTEGER, TempConsoleCHID VARCHAR(64), TempConsoleLeaderMemberID VARCHAR(64), TempConsoleTxInv INTEGER, GroupConsoleOnOff INTEGER, GroupConsoleCHID VARCHAR(64), GroupConsoleGroupID VARCHAR(64), GroupConsoleTxInv INTEGER);
CREATE TABLE tabGPSInformSetting(IntervalForInform INTEGER, listAllFlag INTEGER);
CREATE TABLE tabGPSInformTarget(ChannelID VARCHAR(64),ContactType INTEGER, ContactID VARCHAR(64), OnOff INTEGER DEFAULT 0);
CREATE TABLE [tabGSMBaseSettingPPS] ([IsUseGSM] INTEGER NOT NULL DEFAULT '1', [PIN] CHAR (9) NOT NULL DEFAULT '0000', [Identity] INTEGER NOT NULL DEFAULT '0', [ManualTimeOut] INTEGER NOT NULL DEFAULT '5', [DataFormat] CHAR (32) NOT NULL DEFAULT '#I#S#M', [AutoOffTime] INTEGER NOT NULL DEFAULT '3', [AckSendPath] INTEGER NOT NULL DEFAULT '0', [PagerID] VARCHAR (7) NOT NULL DEFAULT '0000000', [IMEI] VARCHAR (15) NOT NULL DEFAULT '000000000000000', [IMSI] VARCHAR (15) NOT NULL DEFAULT '000000000000000', [GsmFwVersion] VARCHAR (50) NOT NULL DEFAULT '00000000000000000000', [phoneVolumeLevel] INTEGER DEFAULT '60', [phoneMicGainHandsetFront] INTEGER DEFAULT '10', [phoneMicGainHandsetDigital] INTEGER DEFAULT '8192', [retryCount] INTEGER DEFAULT '12', [retryInterval] INTEGER DEFAULT '5000');
CREATE TABLE [tabGSMFuncSettingPPS] ([EmgAckOption] INTEGER NOT NULL DEFAULT '1', [EmgAckSendType] INTEGER NOT NULL DEFAULT '1', [EmgAckMsgContent] CHAR (70) NOT NULL DEFAULT 'Emergency Message!', [EmgVoiceCallOption] INTEGER NOT NULL DEFAULT '0', [EmgVoiceCallPhone] CHAR (20) NOT NULL DEFAULT '+000000000000', [ReceiveMsgBySMS] INTEGER NOT NULL DEFAULT '0', [OnOffDutySendType] INTEGER NOT NULL DEFAULT '0', [EmgMode] INTEGER NOT NULL DEFAULT '0', [GsmMsgReceiveAck] INTEGER NOT NULL DEFAULT '0', [GsmMsgReadAck] INTEGER NOT NULL DEFAULT '0', [GsmMsgManualAck] INTEGER NOT NULL DEFAULT '0');
CREATE TABLE tabGroup (GroupID VARCHAR(64), GroupName VARCHAR(14), GroupNo VARCHAR(18),Note VARCHAR(240),GroupType INTEGER);
CREATE TABLE [tabGroupAddress] ([GroupID] VARCHAR (64) NOT NULL, [Protocoltype] INTEGER NOT NULL, [Address] VARCHAR (30), [SaType] INTEGER NOT NULL, [Freq] INTEGER, [CodeWord] INTEGER, [AlertToneID] INTEGER, [AlertFileName] VARCHAR (25));
CREATE TABLE tabGroupGrouping (
    GroupingID  VARCHAR(64) NOT NULL,
    GroupingAlias VARCHAR(30),
    Type INTEGER NOT NULL,
    Note VARCHAR(240)
);
CREATE TABLE tabGroupMembers (ID VARCHAR(64), GroupID INTEGER, MemberID INTEGER);
CREATE TABLE tabGroupMembers_New (GroupID INTEGER, MemberID INTEGER);
CREATE TABLE tabGroup_New (
    GroupID VARCHAR(64),
    GroupName VARCHAR(14),
    GroupNo VARCHAR(18),
    Note VARCHAR(240),
    RecordFlag INTEGER,
    Type INTEGER,
    Relay INTEGER,
    Priority INTEGER,
    DutyStatus INTEGER,
    AlertMode INTEGER,
    AlertToneID VARCHAR(64) DEFAULT '9999',
    BackgroundColor SMALLINT NOT NULL DEFAULT '0',
    EncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    KeyStrapOption INTEGER NOT NULL DEFAULT 1,
    TwoToneAddr VARCHAR (30),
    FiveToneAddr VARCHAR (30),
    MDCAddr VARCHAR (30),
    UDCAddr VARCHAR (30),
    DMRAddr VARCHAR (30),
    P25Addr VARCHAR (30),
    P25Sub1Addr VARCHAR (30),
    P25Sub2Addr VARCHAR (30),
    P25Sub3Addr VARCHAR (30)
);
CREATE TABLE tabGroupingOfGroup(
	GroupingID  VARCHAR(64) NOT NULL,
	GroupID  VARCHAR(64) NOT NULL
);
CREATE TABLE [tabHomeScreenSettings] ([HomeScreenStyle] INTEGER NOT NULL, [LogoTitleType] INTEGER NOT NULL DEFAULT '1', [LogoFileName] VARCHAR (255), [TitleLineOption] INTEGER NOT NULL DEFAULT '2', [TitleLine1] VARCHAR (30), [TitleLine2] VARCHAR (30), [TitleLine3] VARCHAR (30));
CREATE TABLE tabHomeSettings(
    isFrequencyVisible INTEGER NOT NULL DEFAULT 1,
    StatusBarRssiShowType INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tabImageMessage
(
  ID  VARCHAR(64) NOT NULL,
  MaxVoiceNoteLen INTEGER NOT NULL,
  MaxTextNoteLen INTEGER NOT NULL,
  SplitTime INTEGER NOT NULL,
  SplitRestTime INTEGER NOT NULL,
  EnableAutoResendReq INTEGER NOT NULL,
  CheckResendReqIntv INTEGER NOT NULL,
  ResendDataCount INTEGER NOT NULL,
  HdphotoPercentForShow INTEGER NOT NULL,
  AutoResendReqCount INTEGER NOT NULL,
  ResendReqIntv INTEGER NOT NULL,
  AvailableTime INTEGER NOT NULL,
  PRIMARY KEY(ID)
);
CREATE TABLE tabKVLUSBConnectionSetting (
	ID VARCHAR(64) NOT NULL,
	SubscriberIPAddr NOT NULL DEFAULT '192.168.128.1',
	SubscriberUDPPort NOT NULL DEFAULT '49165',
	KeyLoaderIPAddr NOT NULL DEFAULT '192.168.128.2',
	KeyLoaderUDPPort NOT NULL DEFAULT '49165',
	primary key (ID)
);
CREATE TABLE tabKeypadLockSetting (
    AutoKeypadLock            INTEGER NOT NULL,
    KeypadLockComboDetectTime INTEGER,
    PlayKeyDepressTime        INTEGER
);
CREATE TABLE [tabKnobChAnnouncement] ([ID] INTEGER PRIMARY KEY, [VoicePromptAlias] VARCHAR (25), [VoicePromptFileName] VARCHAR (256), [CHVoiceFileName] VARCHAR (25));
CREATE TABLE [tabLeaderFunctionList] ([FunctionID] INTEGER, [Enable] INTEGER);
CREATE TABLE tabM1DeviceType(
    MemberID VARCHAR(64) ,
    DeviceType INTEGER,
    WaitTime INTEGER
);
CREATE TABLE tabM1MyDeviceType
(
 DeviceType INTEGER
);
CREATE TABLE tabMDC1200ActivationInfo(
    TransmitModeID VARCHAR(64) NOT NULL,
    RepeaterID INTEGER,

    FOREIGN KEY(TransmitModeID) REFERENCES tabTransmitMode(TransmitModeID)
);
CREATE TABLE tabMDCRepeaterSetting(
    ID  VARCHAR(64) NOT NULL,
    MdcPretime INTEGER NOT NULL,
    MdcMsgId INTEGER NOT NULL,
    MdcAutoRepeaterTxType INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabMDCUDCSignaling(
    ID  VARCHAR(64) NOT NULL,
    BasicTxPreTime INTEGER NOT NULL,
    PmbBitSync INTEGER NOT NULL,
    PTTIDType INTEGER NOT NULL,
    InbnMsgRxEnable INTEGER NOT NULL,
    SignallingActivityTimeout INTEGER NOT NULL,
    ACKWaitTimeout INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabMakePhotoSettings(
    PhotoFormat INTEGER,
    ResolutionX INTEGER,
    ResolutionY INTEGER,
    PreviewResolutionX INTEGER,
    PreviewResolutionY INTEGER,
    ColorBit INTEGER,
    ZipParameter INTEGER,
    PreviewZipParameter INTEGER
);
CREATE TABLE tabManDownSetting
(
  ID  VARCHAR(64) NOT NULL,
  TiltOnOff INTEGER NOT NULL,
  TiltLevel INTEGER NOT NULL,
  TiltDuration INTEGER NOT NULL,
  TilePreAlertTime INTEGER NOT NULL,
  StillOnOff INTEGER NOT NULL,
  StillDuration  INTEGER NOT NULL,
  StillPreAlertTime INTEGER NOT NULL,
  FreeFallOnOff INTEGER NOT NULL,
  FreeFallDuration INTEGER NOT NULL,
  FreeFallPreAlertTime INTEGER NOT NULL,
  PRIMARY KEY(ID)
);
CREATE TABLE tabMember (MemberID VARCHAR(64), MemberName VARCHAR(16), MemberType INTEGER, MemberArmyType INTEGER);
CREATE TABLE [tabMemberAddress] ([MemberID] VARCHAR (64) NOT NULL, [Protocoltype] INTEGER NOT NULL, [Address] VARCHAR (30), [AlertToneID] INTEGER, [AlertFileName] VARCHAR (25), PRIMARY KEY ([MemberID], [Protocoltype]));
CREATE TABLE [tabMemberFunctionList] ([FunctionID] INTEGER, [Enable] INTEGER);
CREATE TABLE tabMemberOTAPwd (MemberID VARCHAR(64), Password VARCHAR(6));
CREATE TABLE tabMember_New (
    MemberID VARCHAR(64),
    MemberName VARCHAR(16),
    MemberType INTEGER,
    RadioNo INTEGER,
    RadioType INTEGER,
    Dispatcher INTEGER,
    UserDeviceID VARCHAR(16),
    AlertMode INTEGER,
    BackgroundColor SMALLINT NOT NULL DEFAULT '0',
    AlertToneID VARCHAR(64) DEFAULT '0',
    TwoToneAddr VARCHAR (30),
    FiveToneAddr VARCHAR (30),
    MDCAddr VARCHAR (30),
    UDCAddr VARCHAR (30),
    DMRAddr VARCHAR (30),
    P25Addr VARCHAR (30)
);
CREATE TABLE tabMenuUIType(
   Type INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE tabMfgProductCode (
    ProductSystemCategoryCode VARCHAR(3),
    ProductCategoryCode VARCHAR(3),
    ProductSeriesCode VARCHAR(3),
    ProductSeriesNo INTEGER,
    ProductHardwareCode VARCHAR(16),
    RFBandSingleDual VARCHAR(3),
    RFFirstBandTypeID INTEGER,
    RFSecondBandTypeID INTEGER,
    RFMaxTxPower VARCHAR(3)
);
CREATE TABLE tabModel(
    Model VARCHAR(200)
);
CREATE TABLE tabMultiCHScanList(
    ZoneID  VARCHAR(64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    ScanOrder INTEGER NOT NULL,
    ChannelID VARCHAR(64) NOT NULL
);
CREATE TABLE tabMultiChannelScan(
    ZoneID  VARCHAR(64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    StayTime  INTEGER NOT NULL,
    OpenSpeaker  INTEGER NOT NULL,
    PriorityScanType  INTEGER NOT NULL,
    FirstPriorityCH VARCHAR(64) NOT NULL,
    SecondPriorityCH VARCHAR(64) NOT NULL,
    SamplingFlag INTEGER NOT NULL,
    DwellTime INTEGER NOT NULL,
    PushToListen INTEGER NOT NULL DEFAULT '0',
    Privacy INTEGER NOT NULL DEFAULT '0',
    ResetMode INTEGER DEFAULT '0',
    DelayNTime INTEGER DEFAULT '30',
    EnableCarrierOffTime INTEGER DEFAULT '0',
    CarrierOffDelayTime INTEGER DEFAULT '30',
    AlertMode INTEGER,
    RecordFlag INTEGER,
    DutyStatus INTEGER,
    MatchCTCD INTEGER,
    MatchColorCode INTEGER,
    MatchNAC INTEGER,
    AlwaysUnreadVoiceMessage INTEGER NOT NULL,
	AlwaysUnreadReminderAlert INTEGER NOT NULL,
    Primary key(ZoneID,KnobIndex)
);
CREATE TABLE tabOTAPwd (Password VARCHAR(16));
CREATE TABLE tabOTARConnectionSetting (
	ID VARCHAR(64) NOT NULL,
	KMFName VARCHAR(32) NOT NULL,
	SubscriberIPAddr NOT NULL DEFAULT '0.0.0.0',
	SubscriberOTARPort NOT NULL DEFAULT '64414',
	KMFIPAddr NOT NULL DEFAULT '0.0.0.0',
	KMFUDPPort NOT NULL DEFAULT '64414',
	primary key (ID)
);
CREATE TABLE tabOTARFeatureControlSetting (
	ID VARCHAR(64) NOT NULL,
	P25OTAREnable INTEGER NOT NULL DEFAULT 0,
	P25OTARGenKeyLossKey INTEGER NOT NULL DEFAULT 0,
	P25OTARRadioID INTEGER NOT NULL DEFAULT 1,
	InfiniteKeyRetention INTEGER NOT NULL DEFAULT 0,
	InfiniteUKEKRetention INTEGER NOT NULL DEFAULT 0,
	P25OTARRxSecurityLevel INTEGER NOT NULL DEFAULT 0,
	P25OTARTxSecurityLevel INTEGER NOT NULL DEFAULT 0,
	ResponseKind INTEGER NOT NULL DEFAULT 1,
	TimeBetweenAttempts INTEGER NOT NULL DEFAULT 15,
	UserSelectableRekeyRequest INTEGER NOT NULL DEFAULT 0,
	OTARInactivityTime INTEGER NOT NULL DEFAULT 72,
	RadioInhibitP25OTAR INTEGER NOT NULL DEFAULT 1,
	UserSelectableKeyset INTEGER NOT NULL DEFAULT 0,
	UserSelectableKey INTEGER NOT NULL DEFAULT 0,
  NetworkAddrType INTEGER NOT NULL DEFAULT 0,
  SU_IP VARCHAR(32) NOT NULL DEFAULT '0.0.0.0',
  SU_Port INTEGER NOT NULL DEFAULT 64414,
  PacketDataRegVersion INTEGER NOT NULL DEFAULT 3,
  IPHeaderCompression  INTEGER NOT NULL DEFAULT 0,
  DAC INTEGER NOT NULL DEFAULT 1,
  OTARBtMac VARCHAR(32)  NOT NULL DEFAULT '00:00:00:00:00:00',
	primary key (ID)
);
CREATE TABLE tabOrganization (
				OrgID VARCHAR(64),
				OrgName VARCHAR(14),
				OrgNote VARCHAR(500),
				OrgMembership INTEGER,
				OrgCreateDate Datetime,
				OrgDBVersion INTEGER,
				OrgStep CHAR(1)
);
CREATE TABLE tabOrganization_New (
				ID VARCHAR(64),
				OrgID VARCHAR(64),
				OrgName VARCHAR(14),
				OrgNote VARCHAR(500),
				OrgCreateDate Datetime,
				OrgDBVersion INTEGER,
				OrgStep CHAR(1),
                FinishDate DateTime
);
CREATE TABLE tabOtaSettings(
    ControlChannel VARCHAR(64) NOT NULL
);
CREATE TABLE tabOutRangeSetting(
    outRangeCheckTimer INTEGER NOT NULL DEFAULT 45,
    inRangeRssi        INTEGER NOT NULL,
    inRangeBER 		   INTEGER NOT NULL,
    outRangeCheckCount INTEGER NOT NULL,
    openRfTimer        INTEGER default 30 NOT NULL,
    closeRfTimer       INTEGER default 30 NOT NULL,
    AlertRepeatedly    INTEGER  NOT NULL,
    AlertDuration      INTEGER NOT NULL,
    AlertInterval      INTEGER NOT NULL,
    inRangeCheckTime   INTEGER default 5 NOT NULL,
	InOutRangeAlert    INTEGER DEFAULT 1 NOT NULL
);
CREATE TABLE tabP25RampCodeParam(
    TransmitModeID VARCHAR(64) NOT NULL,
    RxNAC SMALLINT NOT NULL Default 0,
    TxNAC SMALLINT NOT NULL Default 0,

    FOREIGN KEY(TransmitModeID) REFERENCES tabTransmitMode(TransmitModeID)
);
CREATE TABLE tabP25Signaling(
    ID  VARCHAR(64) NOT NULL,
    TXPreTime INTEGER NOT NULL,
    NoSigActTimeout INTEGER NOT NULL,
    ACKWaitTimeout INTEGER NOT NULL,
    MFID INTEGER NOT NULL,
    AlgId INTEGER NOT NULL,
	MinShortDigitalToneDuration  SMALLINT NOT NULL,
    MinLongDigitalToneDuration  SMALLINT NOT NULL,
    MinToneAddrGap  SMALLINT NOT NULL,
	OTAToneFlag   SMALLINT NOT NULL,
    ShortDigitalToneATxDuration  SMALLINT default 1000 NOT NULL,
    ShortDigitalToneBTxDuration  SMALLINT default 2000 NOT NULL,
    LongDigitalToneTxDuration  SMALLINT default 5000 NOT NULL,
    TxToneAddrGap  SMALLINT default 1500 NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabP25TCallAlertEnable(
    ChannelID VARCHAR (64) NOT NULL,
    EnableCallAlert INTEGER NOT NULL
);
CREATE TABLE tabP25TCallAlertSUID(
    ChannelID VARCHAR (64) NOT NULL,
    SUID VARCHAR (30) NOT NULL,
	CallAlertTimeout INTEGER DEFAULT 30 NOT NULL,
	TGIDStayTime INTEGER DEFAULT 30 NOT NULL
);
CREATE TABLE tabP25TCallAlertSUIDGroup(
    ChannelID VARCHAR (64) NOT NULL,
    SUID  VARCHAR (30) NOT NULL,
    GroupID VARCHAR (64) NOT NULL,
    Enable INTEGER DEFAULT 0 NOT NULL,
    Alt INTEGER DEFAULT 0 NOT NULL
);
CREATE TABLE tabP25TChannelRevertSubGroup(
    ChannelID VARCHAR (64) NOT NULL,
    GroupID VARCHAR (64) NOT NULL,
    SubGroupID VARCHAR (64) NOT NULL,
    SeqNo INTEGER NOT NULL,
    OffDuty INTEGER,
    Priority INTEGER,
    OutOfRange INTEGER,
    OTA INTEGER,
    VoiceRecvAck INTEGER,
    VoiceReadAck INTEGER,
    VoiceManualAck INTEGER,
    TextRecvAck INTEGER,
    TextReadAck INTEGER,
    TextManualAck INTEGER,
    ImageRecvAck INTEGER,
    ImageReadAck INTEGER,
    ImageManualAck INTEGER,
    Alt INTEGER
);
CREATE TABLE tabP25TChannelSubGroup(
    ChannelID VARCHAR (64) NOT NULL,
    GroupID VARCHAR (64) NOT NULL,
    SubGroupID VARCHAR (64) NOT NULL,
    SeqNo INTEGER NOT NULL,
    OffDuty INTEGER,
    Priority INTEGER,
    OutOfRange INTEGER,
    OTA INTEGER,
    VoiceRecvAck INTEGER,
    VoiceReadAck INTEGER,
    VoiceManualAck INTEGER,
    TextRecvAck INTEGER,
    TextReadAck INTEGER,
    TextManualAck INTEGER,
    ImageRecvAck INTEGER,
    ImageReadAck INTEGER,
    ImageManualAck INTEGER,
    Alt INTEGER
);
CREATE TABLE tabP25TKnobGroup(
    ZoneID VARCHAR(64) not null,
    KnobIndex INTEGER NOT NULL,
    TransmitModeID VARCHAR(64) NOT NULL,
    GroupID VARCHAR(64) NOT NULL
);
CREATE TABLE tabP25TSignaling (
    ID VARCHAR(64),
    ACKWaitTimeout INTEGER,
    RfssRspWaitTimeout INTEGER,
    MFID INTEGER,
    AlgId INTEGER,
    MinShortDigitalToneDuration  SMALLINT NOT NULL,
    MinLongDigitalToneDuration  SMALLINT NOT NULL,
    MinToneAddrGap  SMALLINT NOT NULL,
    OTAToneFlag   SMALLINT NOT NULL,
    ShortDigitalToneATxDuration  SMALLINT default 1000 NOT NULL,
    ShortDigitalToneBTxDuration  SMALLINT default 2000 NOT NULL,
    LongDigitalToneTxDuration  SMALLINT default 5000 NOT NULL,
    TxToneAddrGap  SMALLINT default 1500 NOT NULL,
	PRIMARY KEY(ID)
);
CREATE TABLE tabP25TSubGroupEnable(
    ChannelID VARCHAR (64) NOT NULL,
    GroupID VARCHAR (64) NOT NULL,
	Enable2TSubGroup INTEGER NOT NULL
);
CREATE TABLE tabP25TrunkingMode(
    TransmitModeID VARCHAR(64) NOT NULL,
    WacnID INTEGER NOT NULL Default 2,
    SystemID INTEGER NOT NULL Default 1,
    WacnSystemIDCheckSkipFlag INTEGER NOT NULL Default 0,
    FullSpectrumScanFlag INTEGER NOT NULL Default 0,
    FullSpectrumScanTimer INTEGER NOT NULL Default 1,
    FullSpectrumScanRanges INTEGER NOT NULL Default 1,
    StartFreqOfRange1 INTEGER,
    EndFreqOfRange1 INTEGER,
    StepSizeOfRange1 INTEGER,
    StartFreqOfRange2 INTEGER,
    EndFreqOfRange2 INTEGER,
    StepSizeOfRange2 INTEGER,
    StartFreqOfRange3 INTEGER,
    EndFreqOfRange3 INTEGER,
    StepSizeOfRange3 INTEGER
);
CREATE TABLE tabP25TrunkingSecurity(
    P25TrunkingID VARCHAR( 64 ),
    DBFileName VARCHAR( 64 ),
    SystemName VARCHAR(14) NOT NULL,
    WacnID  INTEGER NOT NULL,
    SystemID INTEGER NOT NULL,
    SiteCount INTEGER NOT NULL,
    ControlCHCount INTEGER NOT NULL,
    TGIDCount INTEGER NOT NULL,
    SubGroupCount INTEGER NOT NULL,
    IndividualCount INTEGER NOT NULL,
    SUID  VARCHAR(32),
    SystemCreateType INTEGER NOT NULL,
    SystemSource VARCHAR(256),
    BandTypeCode VARCHAR(16),
    Primary key(P25TrunkingID)
);
CREATE TABLE tabP25TrunkingSystemKnob(
    ZoneID VARCHAR (64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    ChannelID VARCHAR (64) NOT NULL,
    P25TrunkingID VARCHAR (64) NOT NULL,
    ProtocolSettingID VARCHAR (64) NOT NULL,
    TxPowerLevel INTEGER NOT NULL,
    AudioSquelch INTEGER NOT NULL,
    OpenSpeakerLogic INTEGER NOT NULL,
    DispatchCHFlag INTEGER NOT NULL DEFAULT '0',
    ATGID INTEGER NOT NULL DEFAULT '0',
    HangTime INTEGER NOT NULL,
    RFPerformance INTEGER NOT NULL DEFAULT '1',
    PushToListen INTEGER NOT NULL DEFAULT '0',
    Privacy INTEGER NOT NULL DEFAULT '0',
    PriorityGroup INTEGER NOT NULL DEFAULT '0',
    VoiceBuffering INTEGER NOT NULL,
	VoiceBufferPrompt INTEGER NOT NULL DEFAULT '0',
	VoiceBufferingForceDrop INTEGER NOT NULL DEFAULT 0,
    AlertMode INTEGER,
    RecordFlag INTEGER,
    DutyStatus INTEGER,
    MSELCallSamplingFlag INTEGER NOT NULL DEFAULT '0',
    MaxMSELSamplingTime INTEGER NOT NULL DEFAULT '2',
    ResetMode INTEGER DEFAULT '0',
    DelayNTime INTEGER DEFAULT '30',
	EnableTgidStayTime SMALLINT DEFAULT 0,
    TgidStayTime  SMALLINT DEFAULT 30,
	AlwaysUnreadVoiceMessage INTEGER NOT NULL,
	AlwaysUnreadReminderAlert INTEGER NOT NULL,
	EncryptedCallFlag INTEGER NOT NULL,
    EncryptionBy INTEGER DEFAULT 0 NOT NULL,
    EncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    KeyStrapOption INTEGER NOT NULL DEFAULT 1,
    KMFSystemID VARCHAR(64) NOT NULL DEFAULT '000000000000',
    PrivateEncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    PrivateKeyStrapOption INTEGER NOT NULL DEFAULT 1,
    ATGEncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    ATGKeyStrapOption INTEGER NOT NULL DEFAULT 1,
    UserSelectableKeyset INTEGER NOT NULL DEFAULT 0,
	UserSelectableKey INTEGER NOT NULL DEFAULT 0,
    DynRegroupEncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    DynRegroupKeyStrapOption INTEGER NOT NULL DEFAULT 1,
    DynRegroupATGEncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    DynRegroupATGKeyStrapOption INTEGER NOT NULL DEFAULT 1,
    DelayNFeature INTEGER DEFAULT '0',
    DelayNTimeEx INTEGER DEFAULT '5',
    EnableCarrierOffTime INTEGER DEFAULT '0',
    CarrierOffDelayTime INTEGER DEFAULT '5',
    PRIMARY KEY (ZoneID, KnobIndex)
);
CREATE TABLE tabPINPasswordSetting(
    ID                INTEGER NOT NULL,
    BootingLockEnable INTEGER,
    KeypadLockEnable  INTEGER,
    AdminPINPwd    VARCHAR(64),
    UserPINPwd     VARCHAR(64),
    MaxErrorCount     INTEGER,
    ErrorDisableTime  INTEGER,
    primary key (ID)
);
CREATE TABLE tabPPSVersion ( MainVersion INTEGER NOT NULL , SubVersion INTEGER NOT NULL , Comment VARCHAR ( 1024 ) );
CREATE TABLE tabPagerSUID(
    P25TAddr VARCHAR (30)
);
CREATE TABLE tabPowerLevel(
    ModelID INTEGER NOT NULL,
    SettingID INTEGER NOT NULL,
    UIName VARCHAR(64),
    SettingType INTEGER,
    Value INTEGER
);
CREATE TABLE [tabPowerSavingRxOff] ([DefaultTime] INTEGER, [TwoTone] INTEGER, [DMR] INTEGER, [P25C] INTEGER, [P25T] INTEGER);
CREATE TABLE tabPowerSavingSetting(
    PowerSavingRxDur INTEGER DEFAULT 100 NOT NULL,
    TwoTone INTEGER DEFAULT 600 NOT NULL,
    FiveTone INTEGER DEFAULT 0 NOT NULL,
    MDCUDC INTEGER DEFAULT 100 NOT NULL,
    DMR INTEGER DEFAULT 0 NOT NULL,
    P25C INTEGER DEFAULT 0 NOT NULL,
    P25T INTEGER DEFAULT 0 NOT NULL
);
CREATE TABLE [tabPpsSecureProtect] ([PpsSecureEnable] INTEGER, [PpsSecurePassword] VARCHAR (10), [PpsLicenseEnable] INTEGER, [PpsLicenseCode] VARCHAR (64), [PpsDeviceProtectEnable] INTEGER, [PpsDeviceProtectCode] VARCHAR (108));
CREATE TABLE tabProductAppearance (
    ProductColor VARCHAR(3),
    ProductPackage VARCHAR(3)
);
CREATE TABLE tabProtocolMode (
    CommunicationMode VARCHAR(3),
    CarrierSquelch INTEGER NOT NULL DEFAULT 0,
    CtcssCdcss INTEGER NOT NULL DEFAULT 0,
    TwoTone INTEGER NOT NULL DEFAULT 0,
    FiveTone INTEGER NOT NULL DEFAULT 0,
    DTMF INTEGER NOT NULL DEFAULT 0,
    MDC1200 INTEGER NOT NULL DEFAULT 0,
    ARDS INTEGER NOT NULL DEFAULT 0,
    DVOA INTEGER NOT NULL DEFAULT 0,
    P25C INTEGER NOT NULL DEFAULT 0,
    DMRC INTEGER NOT NULL DEFAULT 0,
    P25T INTEGER NOT NULL DEFAULT 0,
    DMRT INTEGER NOT NULL DEFAULT 0,
    MPT1327 INTEGER NOT NULL DEFAULT 0,
    POCSAG INTEGER NOT NULL DEFAULT 0,
    FLEX INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tabProtocolSetting(
    ID  VARCHAR(64) NOT NULL,
    ProtocolNo  INTEGER NOT NULL,
    ProtocolName VARCHAR(30),
    ProtocolType INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabPublicInfo
(
  CreateBy INTEGER NOT NULL,
  PreProgramDate DateTime
);
CREATE TABLE tabReceivingMode(
    ZoneID  VARCHAR(64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    SupportedMode INTEGER NOT NULL,
    CurrentMode INTEGER NOT NULL,
    Primary key(ZoneID,KnobIndex)
);
CREATE TABLE tabReceivingMode_NewG4(
    ZoneID VARCHAR (64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    CurrentMode INTEGER NOT NULL,
    ChannelName VARCHAR (30) NOT NULL,
    AlertToneID INTEGER,
    AlertFileName VARCHAR (25),
    PRIMARY KEY (ZoneID, KnobIndex)
);
CREATE TABLE tabRecvBackgroundColorSetting (
	FlashOnTime      SMALLINT NOT NULL DEFAULT '1',
	FlashOffTime     SMALLINT NOT NULL DEFAULT '1',
	FlashDuration    SMALLINT NOT NULL DEFAULT '0',
	IsVoiceFlash     SMALLINT NOT NULL DEFAULT '0',
	IsTextFlash      SMALLINT NOT NULL DEFAULT '0',
	IsImageFlash     SMALLINT NOT NULL DEFAULT '0'
);
CREATE TABLE tabRecvSaList(
    ChannelID VARCHAR (64) NOT NULL,
    SigType INTEGER  NOT NULL,
    SaType  INTEGER DEFAULT 0 NOT NULL,
    ctFreq  INTEGER,
    ctSte   INTEGER,
    cdCodeWord INTEGER,
    cdSte   INTEGER,
    cdModSense INTEGER
);
CREATE TABLE [tabReminderAlertSettingPPS] ([option] INT PRIMARY KEY NOT NULL DEFAULT '0', [ReminderAlertInterval] INTEGER NOT NULL DEFAULT '30
                                  ', [ReminderAlertTimeout] INTEGER NOT NULL DEFAULT '30', [ReminderAlertDuration] INTEGER NOT NULL DEFAULT '5');
CREATE TABLE tabRemoteMonitorSettings (
SettingID INTEGER,
MonitorTime INTEGER,
AutoRecord INTEGER,
IsSilence INTEGER
);
CREATE TABLE tabRepeaterMode (
    TransmitModeID VARCHAR(64) NOT NULL,
    RepeaterType INTEGER default 0 NOT NULL,
    AnalogEnableFlag INTEGER default 0 NOT NULL,
    DMREnableFlag INTEGER default 0 NOT NULL,
    P25EnableFlag INTEGER default 0 NOT NULL,
    RepeaterDevice INTEGER default 0 NOT NULL,
    RepeaterName VARCHAR(14) NOT NULL,
    BandType INTEGER NOT NULL,
    RxFreq INTEGER NOT NULL,
    TxFreq INTEGER NOT NULL,
    RxBandWide INTEGER default 0 NOT NULL,
    TxBandWide INTEGER default 0 NOT NULL,
    HangTime INTEGER default 10 NOT NULL,
    ActivationType INTEGER default 0 NOT NULL,
    MinRSSI INTEGER default 0 NOT NULL,
    AutoDetectEnable INTEGER default 0 NOT NULL,
    DetectTime INTEGER default 10 NOT NULL,
    RoamingType INTEGER default 0 NOT NULL,
    TxRxReverseEnable INTEGER default 0 NOT NULL,

    FOREIGN KEY(TransmitModeID) REFERENCES tabTransmitMode(TransmitModeID)
);
CREATE TABLE tabResendSetting ( option INT NOT NULL DEFAULT '0' , ip VARCHAR ( 40 ) NOT NULL DEFAULT '59.124.64.98' , port VARCHAR ( 8 ) NOT NULL DEFAULT '9105' , retryTimes INTEGER NOT NULL DEFAULT '3' , retryInterval INTEGER NOT NULL DEFAULT '30' , sleepTime INTEGER NOT NULL DEFAULT '10' , resendretryTime INTEGER NOT NULL DEFAULT '120
                                           ' , resendretryTimes INTEGER NOT NULL DEFAULT '1
                                           ' , msgRetrieveOnPowerOn BOOLEAN NOT NULL DEFAULT '0' , msgRetrieveOnInRangeACK BOOLEAN NOT NULL DEFAULT '0' , msgRetrieveWithTimeStmp BOOLEAN NOT NULL DEFAULT '0' );
CREATE TABLE tabRingModeSettingPPS(
    id INTEGER PRIMARY KEY UNIQUE,
    isOn INTEGER NOT NULL DEFAULT '0',
    mode INTEGER NOT NULL DEFAULT '0',
    time INTEGER NOT NULL DEFAULT '2',
    IntervalTime INTEGER NOT NULL DEFAULT '1',
    IsVibInterval INTEGER NOT NULL DEFAULT '0',
    VibIntervalOnTime INTEGER NOT NULL DEFAULT '1',
    VibIntervalOffTime INTEGER NOT NULL DEFAULT '1',
    isVoiceStartStopRing INTEGER NOT NULL DEFAULT '0',
    TwoToneAlertType SMALLINT NOT NULL DEFAULT '0',
    TwoToneAlertDuration INTEGER NOT NULL DEFAULT '2000',
    FiveToneMdcAlertDuration INTEGER NOT NULL DEFAULT '2000',
    DigitalAlertDuration INTEGER NOT NULL DEFAULT '2000'
);
CREATE TABLE [tabSMSSettingPPS] ([Phone] CHAR (20) NOT NULL DEFAULT '+000000000000', [EncodeFormat] INTEGER NOT NULL DEFAULT '1');
CREATE TABLE tabSaInfo(
    TransmitModeID VARCHAR(64) NOT NULL,
    SaType SMALLINT NOT NULL,
    TxRx SMALLINT NOT NULL,
    Freq SMALLINT NOT NULL,
    CodeWord SMALLINT NOT NULL,
    ModSense SMALLINT NOT NULL,
    Ste SMALLINT NOT NULL,

    FOREIGN KEY(TransmitModeID) REFERENCES tabTransmitMode(TransmitModeID)
);
CREATE TABLE tabSingleChannel(
    ZoneID  VARCHAR(64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    ChannelID VARCHAR(64) NOT NULL,
    DefaultPTTGroupID INTEGER NOT NULL,
    Primary key(ZoneID,KnobIndex)
);
CREATE TABLE tabSingleConvCHKnob(
    ZoneID VARCHAR (64) NOT NULL,
    KnobIndex INTEGER NOT NULL,
    ChannelID VARCHAR (64) NOT NULL,
    ConvFreqID VARCHAR (64) NOT NULL,
    ProtocolSettingID VARCHAR (64) NOT NULL,
    RxSaType INTEGER NOT NULL,
    RxCTFreq INTEGER NOT NULL,
    RxCDCodeWord INTEGER NOT NULL,
    RxCDModSense INTEGER NOT NULL,
    TxSaType INTEGER NOT NULL,
    TxCTFreq INTEGER NOT NULL,
    TxCDCodeWord INTEGER NOT NULL,
    TxCDModSense INTEGER NOT NULL,
    RxNAC INTEGER NOT NULL,
    TxNAC INTEGER NOT NULL,
    RxColorCode INTEGER NOT NULL,
    TxColorCode INTEGER NOT NULL,
    Emphasis INTEGER NOT NULL,
    Compander INTEGER NOT NULL,
    TxPowerLevel INTEGER NOT NULL,
    AudioSquelch INTEGER NOT NULL,
    OpenSpeakerLogic INTEGER NOT NULL,
    HangTime INTEGER NOT NULL,
    DefaultGroupID VARCHAR (64),
    DispatchCHFlag INTEGER NOT NULL DEFAULT '0',
    Encryption INTEGER NOT NULL DEFAULT '0',
    RFPerformance INTEGER NOT NULL DEFAULT '1',
    PushToListen INTEGER NOT NULL DEFAULT '0',
    Privacy INTEGER NOT NULL DEFAULT '0',
    SquelchOption INTEGER NOT NULL,
    VoiceBuffering INTEGER NOT NULL,
    VoiceBufferPrompt INTEGER NOT NULL DEFAULT '0',
    VoiceBufferingForceDrop INTEGER NOT NULL DEFAULT 0,
    ResetMode INTEGER DEFAULT '0',
    DelayNTime INTEGER DEFAULT '30',
    EnableCarrierOffTime INTEGER DEFAULT '0',
    CarrierOffDelayTime INTEGER DEFAULT '30',
    AlertMode INTEGER,
    RecordFlag INTEGER,
    DutyStatus INTEGER,
    TransmitMode INTEGER NOT NULL,
    AlwaysUnreadVoiceMessage INTEGER NOT NULL,
	AlwaysUnreadReminderAlert INTEGER NOT NULL,
    DelayNFeature INTEGER DEFAULT '0',
    DelayNTimeEx INTEGER DEFAULT '30',
	EnableTgidStayTime SMALLINT DEFAULT 0,
    TgidStayTime  SMALLINT DEFAULT 30,
    EncryptionBy INTEGER DEFAULT 0 NOT NULL,
    EncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    KeyStrapOption INTEGER NOT NULL DEFAULT 1,
    KMFSystemID VARCHAR(64) NOT NULL DEFAULT '000000000000',
    PrivateEncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    PrivateKeyStrapOption INTEGER NOT NULL DEFAULT 1,
    UserSelectableKeyset INTEGER NOT NULL DEFAULT 0,
	UserSelectableKey INTEGER NOT NULL DEFAULT 0,
	EncryptedCallFlag INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (ZoneID, KnobIndex)
);
CREATE TABLE tabSingleToneActivationInfo(
    TransmitModeID VARCHAR(64) NOT NULL,
    frequency INTEGER NOT NULL,

    FOREIGN KEY(TransmitModeID) REFERENCES tabTransmitMode(TransmitModeID)
);
CREATE TABLE tabSpecialKeyActionTime(
    KeyCode    INTEGER NOT NULL,
    KeyName    VARCHAR,
    ActionTime INTEGER NOT NULL,
    primary key (KeyCode)
);
CREATE TABLE tabTDMARepeater(
    AlertThreshold INTEGER NOT NULL,
    MissingTimes INTEGER NOT NULL,
    AckWaitTimeOut INTEGER NOT NULL,
    RRNotifyTimes INTEGER NOT NULL,
    RRZoneNo VARCHAR(64)
);
CREATE TABLE tabTGIDHoldAndRemoveSetting (
	TGHoldFunc INTEGER NOT NULL DEFAULT '0',
	TGRemvFunc INTEGER NOT NULL DEFAULT '0',
    TGHoldTimeout INTEGER NOT NULL DEFAULT '0',
    UnholdAlert SMALLINT NOT NULL DEFAULT '0'
);
CREATE TABLE [tabTextMessage] ([ID] VARCHAR (64) NOT NULL, [MaxLen] INTEGER NOT NULL, [IsAutoShow] INTEGER);
CREATE TABLE tabTodaySettings(
   isAlertToneOn INTEGER NOT NULL DEFAULT 1,
   AlertToneInterval INTEGER NOT NULL DEFAULT 180
);
CREATE TABLE tabTowTruckSetting (
    CradleType        SMALLINT  NOT NULL  DEFAULT '0',
    PowerControlMode  SMALLINT  NOT NULL  DEFAULT '0'
);
CREATE TABLE tabTransmitMode (
    ID VARCHAR(64) NOT NULL,
    Name VARCHAR(14) NOT NULL,
    TransmitMode INTEGER default 0 NOT NULL,

    PRIMARY KEY(ID)
);
CREATE TABLE tabTransmitModeArea (
    TransmitModeID VARCHAR(64) NOT NULL,
    AreaName VARCHAR(14) NOT NULL,
    TopLeftLatitude REAL,
    TopLeftLongitude REAL,
    BottomRightLatitude REAL,
    BottomRightLongitude REAL,
    AreaType INTEGER default 0 NOT NULL,
    Note VARCHAR(150),

    FOREIGN KEY(TransmitModeID) REFERENCES tabTransmitMode(TransmitModeID)
);
CREATE TABLE tabTrunkingMode (
    TransmitModeID VARCHAR(64) NOT NULL,
    TrunkingType  INTEGER default 0 NOT NULL,

    FOREIGN KEY(TransmitModeID) REFERENCES tabTransmitMode(TransmitModeID)
);
CREATE TABLE tabTwoToneSignaling (
    ID VARCHAR (64) NOT NULL,
    Pretime INTEGER NOT NULL,
    Tone1TxDuration INTEGER NOT NULL,
    Gap1TxDuration INTEGER NOT NULL,
    Tone2TxDuration INTEGER NOT NULL,
    Gap2TxDuration INTEGER NOT NULL,
    LongToneDuration INTEGER NOT NULL,
    LongToneTxGap INTEGER NOT NULL,
    LongToneRxTmout INTEGER NOT NULL,
    SignallingActivityTimeout INTEGER NOT NULL,
    RxTimeout INTEGER NOT NULL,
    FreqSetID VARCHAR (64),
    MinToneDuration SMALLINT default 400 NOT NULL,
    ThresholdToneDetector SMALLINT default 0 NOT NULL,
    NSigPwrOnThreshold INTEGER default 1 NOT NULL,
    NSigPwrOffThreshold INTEGER default 1 NOT NULL,
    WSigPwrOnThreshold INTEGER default 1 NOT NULL,
    WSigPwrOffThreshold INTEGER default 1 NOT NULL,
    PRIMARY KEY (ID)
);
CREATE TABLE tabUiNumberFormat(
		CallIdShowType SMALLINT NOT NULL,
		SysIdShowType  SMALLINT NOT NULL
);
CREATE TABLE tabVoiceCallSetting (
       SettingID INTEGER NOT NULL,
       CallTimeLimitOption INTEGER NOT NULL,
       UserDefineCallTimeLimit INTEGER NOT NULL,
       CallTimeLimitChangeTime INTEGER  NOT NULL,
       PenaltyTime INTEGER NOT NULL,
       CallBackTime INTEGER NOT NULL,
       DetectCHFree INTEGER NOT NULL,
       LeaderImpolite INTEGER NOT NULL,
       SideTone INTEGER NOT NULL,
       AutoMute INTEGER NOT NULL,
       WisperOnOff INTEGER NOT NULL,
       EchoCannelOnOff INTEGER NOT NULL,
       NPPOnOff INTEGER NOT NULL,
       PRIMARY KEY(SettingID)
);
CREATE TABLE tabVoiceCallVocoderSetting (
    DMRVocoder INTEGER NOT NULL DEFAULT 0,
    DMRCompatibility INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tabVoiceMemoSetting (
    ID INTEGER NOT NULL DEFAULT 1,
    MaxNumber INTEGER NOT NULL DEFAULT 198,
    RecordTime INTEGER NOT NULL DEFAULT 60,
    AlertToneDur INTEGER NOT NULL DEFAULT 15,
    PlayOption INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(ID)
);
CREATE TABLE tabVoiceStorageSetting(
    MaxRecordTime INTEGER DEFAULT 600
);
CREATE TABLE tabZone(
    ID  VARCHAR(64) NOT NULL,
    Name VARCHAR(30) NOT NULL,
    Note VARCHAR(100) DEFAULT NULL,
    ModifyByDevice INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabZone_New(
    ID  VARCHAR(64) NOT NULL,
    ZoneNo INTEGER NOT NULL,
    Name VARCHAR(30) NOT NULL,
    Note VARCHAR(100) DEFAULT NULL,
    ModifyByDevice INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
