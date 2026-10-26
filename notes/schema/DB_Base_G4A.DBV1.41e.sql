CREATE TABLE TabImageBoxSetting(
                   SettingId  SMALLINT ,
                   EnableAutoResendReq INTEGER,
                   AutoResendReqCount INTEGER,
                   ResendReqIntv INTEGER,
                   CheckResendReqIntv INTEGER,
                   ResendDataCount INTEGER,
                   AvailableTime INTEGER,
                   HandleOrgPic INTEGER,
                   HdphotoPercentForShow INTEGER,
                   MaxTextNoteLen INTEGER,
                   PRIMARY KEY(SettingId)
);
CREATE TABLE TabInformation(
  ID INTEGER NOT NULL,
  Description VARCHAR(200),
  ItemIndex INTEGER NOT NULL,
  Enable INTEGER NOT NULL
);
CREATE TABLE TabRRSettings(
   AlertThreshold INTEGER,
   MissingTimes INTEGER,
   AckWaitTimeOut INTEGER,
   RRNotifyTimes INTEGER,
   RRZoneNo INTEGER,
   isRepeater INTEGER,
   switch  INTEGER,
   checkSwitch INTEGER
);
CREATE TABLE tab5ToneRepeaterSetting(
   SACID  INTEGER NOT NULL,
   DigitNum    SMALLINT,
   ToneDigit1  SMALLINT,
   ToneDigit2  SMALLINT,
   ToneDigit3  SMALLINT,
   ToneDigit4  SMALLINT,
   ToneDigit5  SMALLINT,
   ToneDigit6  SMALLINT,
   ToneDigit7  SMALLINT,
   RepeaterAccessTxType SMALLINT,
   FirstRepeaterIdTxDuration INTEGER,
   ToneLink1TxDuration  INTEGER,
   PRIMARY KEY (SACID)
);
CREATE TABLE tabAccessPassword(
   ID    INTEGER NOT NULL,
   Enable INTEGER NOT NULL,
   AccessPassword VARCHAR(64),
   PRIMARY KEY(ID)
);
CREATE TABLE tabAlertModeSetting(
    id INTEGER PRIMARY KEY,
    ch_id INTEGER NOT NULL UNIQUE,
    isOn INTEGER NOT NULL DEFAULT '1',
    mode INTEGER NOT NULL DEFAULT '0',
    time INTEGER NOT NULL DEFAULT '2',
    TwoToneAlertType SMALLINT NOT NULL DEFAULT '0',
    TwoToneAlertDuration INTEGER NOT NULL DEFAULT '2000',
    FiveToneMdcAlertDuration INTEGER NOT NULL DEFAULT '2000',
    DigitalAlertDuration INTEGER NOT NULL DEFAULT '2000',
    DependOn INTEGER NOT NULL DEFAULT '0'
);
create table tabAlertToneSettings(
id integer,
SceneID integer,
ToneType integer,
ToneVolume integer,
primary key (id));
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
CREATE TABLE tabAllBandScanSettings (
    SettingId INTEGER,
    ScanParamID1 INTEGER,
    ScanParamID2 INTEGER,
    ScanType INTEGER,
    PRIMARY KEY(SettingId)
);
create table tabAllowAlterZoneChannelsByManualCHSetting(
   ZoneID INTEGER NOT NULL,
   KnobIndex SMALLINT NOT NULL,
   PRIMARY KEY(ZoneID,KnobIndex)
);
CREATE TABLE tabArmyRoleSettings(
    ContactID INTEGER NOT NULL,
    ArmyRole INTEGER NOT NULL,
    PRIMARY KEY(ContactID)
);
create table tabAudioRecordTimer (
function smallint,
attribute smallint,
second smallint,
primary key (function, attribute));
CREATE TABLE tabAudioService
(
    DelayCloseSpeakerTimeOfAMP INTEGER NOT NULL
);
CREATE TABLE tabAudioSquelchSettings(
    id INTEGER PRIMARY KEY UNIQUE,
    SquelchLevel SMALLINT NOT NULL,
    LowPercent INTEGER NOT NULL,
    MediumPercent INTEGER NOT NULL,
    HighPercent INTEGER NOT NULL
);
create table tabAudioToneDetail(
SceneID integer,
indexId integer,
freqA integer,
freqB integer,
dur integer,
primary key (SceneID, indexId));
create table tabBackLightModeSetting(
Mode integer,
BrightnessLevel smallint);
CREATE TABLE tabBacklightManualModeSetting(
    Night INTEGER,
    Day INTEGER,
    Glare INTEGER
);
create table tabBacklightModeSettings(
id integer,
Mode integer,
Brightness smallint,
primary key (id));
create table tabBacklightSettings(
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
CREATE TABLE tabBasicSignalingSetting(
    BasicSelCal  SMALLINT default 1 NOT NULL,
    EmgPTTIDEnable  SMALLINT default 0 NOT NULL,
    EmgPTTID  INTEGER default 0 NOT NULL,
    EmgStickyRevert  SMALLINT default 0 NOT NULL
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
CREATE TABLE tabBleParamSetting (
    BlePair                    INTEGER        NOT NULL,
    UserSettingAdvertiseStatus INTEGER        NOT NULL,
    BleFwVer                   VARCHAR(32),
    BleName                    VARCHAR(13),
    BleAdvertiseTime           INTEGER        NOT NULL
);
CREATE TABLE tabBrightLevelToBrightness(
  BrightnessLevel INTEGER NOT NULL,
  Brightness INTEGER NOT NULL,
  PRIMARY KEY(BrightnessLevel)
);
CREATE TABLE tabCallAlertGroup (
    ChannelID        INTEGER        NOT NULL,
    SUID             INTEGER        NOT NULL,
    Address          VARCHAR( 30 )  NOT NULL,
    CallAlertTimeSec INTEGER  NOT NULL,
    CallAlertTimeMSec INTEGER NOT NULL
);
create table tabCallTimeLimit(
option1 SMALLINT,
option2 SMALLINT,
option3 SMALLINT,
option4 SMALLINT,
defaultCallTimeLimit SMALLINT,
liveTime INTEGER );
CREATE TABLE tabCannedMessageMap(
    MessageID INTEGER NOT NULL,
    MessageContent VARCHAR(250),
    PRIMARY KEY(MessageID)
);
CREATE TABLE tabCap(
   attack INTEGER NOT NULL DEFAULT 0
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
    saturation INTEGER,
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
    ID                   INTEGER NOT NULL,
    Name                 VARCHAR(16) NOT NULL,
    isSupportRepeater SMALLINT  NOT NULL,
    TxChanBandWidth      SMALLINT default 1 NOT NULL,
    RxChanBandWidth      SMALLINT default 1 NOT NULL,
    TXFrequency          INTEGER default 0 NOT NULL,
    RXFrequency          INTEGER default 0 NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabChannelAdvInfo(
    ID  INTEGER NOT NULL,
    ChannelType INTEGER NOT NULL default 0 NOT NULL,
    SignallingSquechLogic SMALLINT default 0 NOT NULL,
    AutoMute  SMALLINT default 0 NOT NULL,
    SideTone  SMALLINT default 0 NOT NULL,
    AudioSquelchType SMALLINT default 0 NOT NULL,
    CarrierSquelchSetting SMALLINT default 80 NOT NULL,
    SINADSquelchSetting SMALLINT default 6 NOT NULL,
    Emphasis     INTEGER default 0 NOT NULL,
    Compander     INTEGER default 0 NOT NULL,
    MsgRetries  INTEGER NOT NULL,
    RFActivityTimeout INTEGER NOT NULL,
    UniLongDataEnable SMALLINT default 1 NOT NULL,
    ColorCode INTEGER,
    NAC INTEGER,
    DefaultCallID INTEGER NOT NULL,
    ATGID INTEGER,
    TaTxSaType  INTEGER default 0 NOT NULL,
    TaTxCtFreq  INTEGER default 0 NOT NULL,
    TaTxCtSte  INTEGER default 0 NOT NULL,
    TaTxCdCodeWord  INTEGER default 0 NOT NULL,
    TaTxCdSte  INTEGER default 0 NOT NULL,
    TaTxCdModSense  INTEGER default 0 NOT NULL,
    SquelchOption INTEGER default 0 NOT NULL,
    PowerSavingRxOffDur INTEGER default 0 NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabChannelAnnouncement (
    ZoneID    INTEGER        NOT NULL,
    ChannelID INTEGER        NOT NULL,
    FileName  VARCHAR( 25 )  NOT NULL
);
CREATE TABLE tabChannelLock (
    zoneId INTEGER not null,
    ChKnobIndex INTEGER not null,
    ChID INTEGER not null,
PRIMARY KEY (zoneId)
);
CREATE TABLE tabCipherKeyAssignment (
    chID INTEGER NOT NULL,
    KeyName VARCHAR(32) NOT NULL,
    CKR INTEGER NOT NULL,
    keyID INTEGER NOT NULL,
    type INTEGER NOT NULL,
    assignmentID INTEGER NOT NULL,
    keyStrappingOption INTEGER NOT NULL,
    secureSwitch INTEGER NOT NULL,
    PRIMARY KEY (chID, type, assignmentID)
);
CREATE TABLE tabContactAddr(
    ContactID  INTEGER NOT NULL,
    SignalingType INTEGER NOT NULL,
    Address  VARCHAR(30) NOT NULL,
    FOREIGN KEY(ContactID) REFERENCES tabContacts(ID),
    FOREIGN KEY(SignalingType) REFERENCES tabSignalingTypes(SignalingType),
    UNIQUE(ContactID,SignalingType)
);
CREATE TABLE tabContactAddrAndAlertTone (
    ContactID     INTEGER,
    SignalingType INTEGER,
    Address       CHAR,
    ToneId        INTEGER,
    FileName      CHAR
);
CREATE TABLE [tabContactOfACK] ([ContactID] INTEGER PRIMARY KEY NOT NULL DEFAULT '1', [TxtMsgReceiveACK] INTEGER NOT NULL DEFAULT '0', [TxtMsgReadACK] INTEGER NOT NULL DEFAULT '0', [TxtMsgManualACK] INTEGER NOT NULL DEFAULT '0', [VoiceMsgReceiveACK] INTEGER NOT NULL DEFAULT '0', [VoiceMsgReadACK] INTEGER NOT NULL DEFAULT '0', [VoiceMsgManualACK] INTEGER NOT NULL DEFAULT '0', [ImageMsgReceiveACK] INTEGER NOT NULL DEFAULT '0', [ImageMsgReadACK] INTEGER NOT NULL DEFAULT '0');
CREATE TABLE tabContactOfBackground (
    ContactID          SMALLINT NOT NULL,
    BackgroundColor    SMALLINT NOT NULL DEFAULT '0'
);
CREATE TABLE tabContactOfChan(
    ChannelID  INTEGER NOT NULL,
   	ContactID  INTEGER NOT NULL,
    SignalingType INTEGER NOT NULL,
    FOREIGN KEY(ChannelID) REFERENCES tabChannel(ID),
    FOREIGN KEY(ContactID) REFERENCES tabContacts(ID),
    PRIMARY KEY(ChannelID, ContactID, SignalingType)
);
CREATE TABLE tabContactOfRelayCtl(
    ContactID INTEGER NOT NULL,
    RelayID INTEGER NOT NULL DEFAULT 0,
    RelayCtrl INTEGER NOT NULL DEFAULT 0,
    RelayOnTime INTEGER NOT NULL DEFAULT 20
);
CREATE TABLE tabContacts(
    ID  INTEGER NOT NULL,
    Name  VARCHAR(30) NOT NULL,
    IDType  INTEGER NOT NULL,
    Role  INTEGER NOT NULL,
    IsUnication SMALLINT NOT NULL,
    Note  VARCHAR(50) ,
    UNIQUE(ID)
);
create table tabCurrentHome
(
    CurrentHomeID integer,
    primary key(CurrentHomeID)
);
CREATE TABLE tabCurrentZone(
   CurrentZoneID INTEGER NOT NULL,
   PRIMARY KEY(CurrentZoneID)
);
CREATE TABLE tabDMRRepeaterSetting(
   SACID  INTEGER NOT NULL,
   TimeSlot  SMALLINT NOT NULL,
   TimingAlign  SMALLINT NOT NULL,
   TxColorCode  SMALLINT NOT NULL,
   RxColorCode  SMALLINT NOT NULL,
   PRIMARY KEY (SACID)
);
CREATE TABLE tabDMRSignaling(
    SigID  INTEGER NOT NULL,
    TXPreFrames INTEGER default 0,
    RxColorCode INTEGER default 0,
    RCRxEnable  INTEGER,
    NoSigActTimeout  INTEGER NOT NULL,
    ACKWaitTimeout    INTEGER NOT NULL,
    MFID        INTEGER,
    SAP4NRS     INTEGER default 4,
    SAP4RS      INTEGER default 5,
    DMRCompatibility INTEGER NOT NULL DEFAULT 0,
    DMRVocoder INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(SigID)
);
CREATE TABLE tabDSPEngineerSettings(
    Enable SMALLINT,
    Parameters VARCHAR(128)
);
CREATE TABLE tabDateTimeSetting (
    date_format INT NOT NULL,
    hour_format INT NOT NULL
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
create table tabDeviceOnOff(
    DeviceID INTEGER NOT NULL,
    DeviceName Varchar,
    OnOff SMALLINT NOT NULL,
    primary key (DeviceID)
);
CREATE TABLE tabDispatchCenterSetting (
    ContactID INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE tabDispatchChannel(
    DispatchChannelID INTEGER,
    PRIMARY KEY(DispatchChannelID)
);
CREATE TABLE tabDispathCenterACK (
    ManualACKKeepTime INTEGER
);
CREATE TABLE tabDispathCenterManualAck(
    id INTEGER,
    text VARCHAR(512),
    PRIMARY KEY(id)
);
CREATE TABLE tabDutySetting(
    ID INTEGER,
    DutyStatus INTEGER,
    DutyCycle INTEGER,
    StartTimeHr INTEGER,
    StartTimeMin INTEGER,
    EndTimeHr INTEGER,
    EndTimeMin INTEGER,
    OffDutyRecord smallint,
    PRIMARY KEY(ID)
);
CREATE TABLE tabEmgRevtChSettings(
    EmgRvtChId   INTEGER NOT NULL,
    AckForIds  VARCHAR(120) ,
    EmgGroupId    INTEGER NOT NULL,
    PRIMARY KEY(EmgRvtChId)
);
CREATE TABLE tabEmgSettings(
    EmgSettingID   INTEGER NOT NULL,
    EmgType   SMALLINT NOT NULL,
    PoliteRetryTimes SMALLINT default 5 NOT NULL,
    ImpoliteRetryTimes SMALLINT default 10 NOT NULL,
    HotMicLength  SMALLINT default 60 NOT NULL,
    ACKTimeLimit  SMALLINT default 10 NOT NULL,
    ZoneID     INTEGER NOT NULL,
    KnobID     INTEGER NOT NULL,
    ChannelID     INTEGER NOT NULL,
    ContactID     INTEGER NOT NULL,
    PromptFileName VARCHAR(128),
    GPSInformInterval INTEGER NOT NULL,
    FOREIGN KEY(ChannelID) REFERENCES tabChannel(ID),
    PRIMARY KEY(EmgSettingID)
);
create table tabEnablePassiveWorkStationFunctions(
FunctionID integer,
Note VARCHAR(30),
primary key (FunctionID)
);
CREATE TABLE tabEnableReceivingFunctions(
    FunctionID INTEGER,
    Note varchar(30),
    PRIMARY KEY(FunctionID)
);
CREATE TABLE tabEncryptionKeyList(
    ID INTEGER NOT NULL,
    KeyID INTEGER NOT NULL,
    KeyAlias VARCHAR(32) NOT NULL,
    CKR INTEGER NOT NULL,
    primary key (ID)
);
Create TABLE tabEncryptionSetting (
    ChannelID  INTEGER NOT NULL,
    EncryptionType SMALLINT NOT NULL,
    PeriodicKeyfailAlert INTEGER NOT NULL DEFAULT 1,
    ClearAlert INTEGER NOT NULL DEFAULT 0,
    ChkSecureSwitchWhenStrap INTEGER NOT NULL DEFAULT 0,
    DisplayOnSecureSwitch INTEGER NOT NULL DEFAULT 2,
    DisplayOnPTT INTEGER NOT NULL DEFAULT 2,
    DisplayOnChannelChange INTEGER NOT NULL DEFAULT 2,
    PRIMARY KEY(ChannelId,EncryptionType)
);
CREATE TABLE tabFiveToneFreqSets(
		SetID  INTEGER NOT NULL,
		Freq0  INTEGER  NOT NULL,
		Freq1  INTEGER  NOT NULL,
		Freq2  INTEGER  NOT NULL,
		Freq3  INTEGER  NOT NULL,
		Freq4  INTEGER  NOT NULL,
		Freq5  INTEGER  NOT NULL,
		Freq6  INTEGER  NOT NULL,
		Freq7  INTEGER  NOT NULL,
		Freq8  INTEGER  NOT NULL,
		Freq9  INTEGER  NOT NULL,
		Freq10  INTEGER  NOT NULL,
		Freq11  INTEGER  NOT NULL,
		Freq12  INTEGER  NOT NULL,
		Freq13  INTEGER  NOT NULL,
		Freq14  INTEGER  NOT NULL,
		Freq15  INTEGER  NOT NULL,
		RepeatToneIdx  INTEGER NOT NULL,
		GroupToneIdx  INTEGER NOT NULL,
		AckToneIdx  INTEGER NOT NULL,
		Link2ToneIdx  INTEGER NOT NULL,
		PRIMARY KEY(SetID)
);
CREATE TABLE tabFiveToneSignaling(
    SigID  INTEGER NOT NULL,
    FreqSetID   INTEGER NOT NULL,
    SrcIdTxFlag  INTEGER NOT NULL,
    Link2TxType  INTEGER NOT NULL,
    AckTxType  INTEGER NOT NULL,
    NormalDigitTxDur   INTEGER NOT NULL,
    FirstSrcIdTxDur   INTEGER NOT NULL,
    FirstDestIdTxDur  INTEGER NOT NULL,
    LinkrTxDur  INTEGER NOT NULL,
    Link2TxDur  INTEGER NOT NULL,
    LinkaTxDur  INTEGER NOT NULL,
    LeadOutTxDur  INTEGER NOT NULL,
    NoSigActTimeout  INTEGER NOT NULL,
    ACKWaitTimeout    INTEGER NOT NULL,
    RxToneGap  INTEGER NOT NULL,
    RxTimeout  INTEGER NOT NULL,
    ACKWaitFlag INTEGER NOT NULL,
    PRIMARY KEY(SigID),
    FOREIGN KEY(FreqSetID) REFERENCES tabFiveToneFreqSets(SetID)
);
create table tabFixAlertToneVolume(
ToneVolume integer
);
create table tabFunLedMap (funId integer, priority integer, ledId integer, colorId integer, primary key (funId));
CREATE TABLE tabFuncAlertToneBaseInfo(
   funcId INTEGER NOT NULL,
   controlByDuty SMALLINT NOT NULL,
   controlByAlertMode INTEGER NOT NULL,
   alias VARCHAR(128),
   PRIMARY KEY(funcId)
);
create table tabFuncClassToneVolume(
    funcClass smallint,
    type smallint,
    volume smallint,
    primary key (funcClass)
);
create table tabFuncLEDSettings (funId integer, period integer, percent integer, cycle integer, interval integer, count integer, primary key (funId));
create table tabFuncSettingPassword(
    password varchar(6),
    onoff INTEGER
);
create table tabFuncStatusPassword(
    password varchar(6),
    onoff INTEGER
);
create table tabFuncTone(
 id integer,
 enable smallint,
 toneID integer,
 alias VARCHAR(128),
 sourceType smallint,
 sourceID smallint,
 type smallint,
 interval integer,
 maxTime integer,
 primary key (id)
);
create table tabFuncToneClass(
 ID smallint,
 funcClass smallint,
 primary key (ID)
);
create table tabFuncToneVolume(
 id integer,
 type smallint,
 volume smallint,
 primary key (id)
);
create table tabFuncVibrate(
 funcId integer,
 Enable smallint,
 VibPatternID integer,
 VibAlias VARCHAR(128),
 type smallint,
 interval integer,
 maxTime integer,
 primary key (funcId)
);
CREATE TABLE tabFunctionListOfUserSetting(
  function_id  INTEGER,
  enabled INTEGER,
  show_type INTEGER
);
create table tabFunctionOnOffOfChan(
    ChannelID INTEGER NOT NULL,
    GPSInformToGroupOn SMALLINT NOT NULL,
    GPSInformToTmpConsoleOn SMALLINT NOT NULL,
    GPSInformToConsoleOn SMALLINT NOT NULL,
    EncryptionOn SMALLINT NOT NULL,
    Override  SMALLINT NOT NULL,
    RAndRMode SMALLINT NOT NULL,
    talkAroundOn SMALLINT NOT NULL,
    privacy SMALLINT NOT NULL,
    pushtolisten SMALLINT NOT NULL,
    PriorityCall SMALLINT NOT NULL,
    DelayPlayVoiceMsg SMALLINT NOT NULL,
    MonitorModeOn SMALLINT NOT NULL,
    VoiceBufferPrompt SMALLINT NOT NULL,
    AlwaysUnreadVoiceMessage SMALLINT NOT NULL,
    AlwaysUnreadReminderAlert SMALLINT NOT NULL,
    TempRecvOn SMALLINT NOT NULL,
    ForceDrop INTEGER NOT NULL,
    primary key (ChannelID)
);
CREATE TABLE tabFunctionParamOfChan(
    ChannelID  INTEGER NOT NULL,
    VoiceCallSetID  INTEGER NOT NULL,
    TxPowerSettingID  INTEGER default 0 NOT NULL,
    rAndRSetID INTEGER default 0 NOT NULL,
    RFPreformanceId INTEGER default 1 NOT NULL,
    ResetModeSetID INTEGER default 0 NOT NULL,
    PRIMARY KEY(ChannelID)
);
CREATE TABLE tabFunctionSettingList(
  SettingItemSerial INTEGER,
  FuncSettingId INTEGER,
  Enable INTEGER
);
CREATE TABLE tabFunctionStatus(
    SettingItemId SMALLINT NOT NULL PRIMARY KEY,
    SettingItemSerial SMALLINT NOT NULL,
    Name varchar(64),
    ItemType SMALLINT NOT NULL,
    HasChild SMALLINT NOT NULL,
    BelongSettingItem SMALLINT DEFAULT NULL,
    Status SMALLINT
);
CREATE TABLE tabFunctionsOnOff (
    AllBandScanOn SMALLINT NOT NULL,
    StunOn              SMALLINT NOT NULL,
    ChannelLockOn       SMALLINT NOT NULL,
    GpsDisplayOn        SMALLINT NOT NULL,
    GpsMonitorOn        SMALLINT NOT NULL,
    Alarm1On            SMALLINT NOT NULL,
    Alarm2On            SMALLINT NOT NULL,
    backlightOn         SMALLINT NOT NULL,
    CallLimitRelieve    SMALLINT NOT NULL,
    MonitorModeOn       SMALLINT NOT NULL,
    ForestFireSupportOn SMALLINT NOT NULL,
    LocationRecordOn    SMALLINT NOT NULL,
    DutyOn              SMALLINT NOT NULL,
    ReminderAlertOn     SMALLINT NOT NULL,
    p25TAudioAcceptOn   SMALLINT NOT NULL,
    OutOfRangeOn        SMALLINT NOT NULL,
    InTrafficCHOn       SMALLINT NOT NULL,
    InScanModeOn        SMALLINT NOT NULL,
    PoorVoiceCallOn     SMALLINT NOT NULL,
    AudioSquelchOn      SMALLINT NOT NULL,
    onSiteOn            SMALLINT NOT NULL,
    GpsInformOn         SMALLINT NOT NULL,
    UnreadAlertOn       SMALLINT NOT NULL,
    FixAlert            INTEGER  NOT NULL DEFAULT '0',
    powsavLedOn         INTEGER  NOT NULL DEFAULT '1',
    StartupLEDOn        SMALLINT NOT NULL,
    ChargeLEDOn         SMALLINT NOT NULL,
    StandbyLEDOn        SMALLINT NOT NULL,
    LowPowerLEDOn       SMALLINT NOT NULL,
    ReceiveLEDOn        SMALLINT NOT NULL,
    SysExceptionLEDOn   SMALLINT NOT NULL,
    UnreadLEDOn         SMALLINT NOT NULL,
    DspExceptionLEDOn   SMALLINT NOT NULL,
    BtStartupLEDOn      SMALLINT NOT NULL,
    BtTransmitLEDOn     SMALLINT NOT NULL,
    BtExceptionLEDOn    SMALLINT NOT NULL,
    GsmStartupLEDOn     SMALLINT NOT NULL,
    GsmTransmitLEDOn    SMALLINT NOT NULL,
    GsmExceptionLEDOn   SMALLINT NOT NULL,
    ReceiveRSLEDOn      SMALLINT NOT NULL,
    showMsgContentOn    SMALLINT NOT NULL,
    ChargeFullLEDOn     SMALLINT NOT NULL,
    LowChargeLEDOn      SMALLINT NOT NULL,
    ChargeErrLEDOn      SMALLINT NOT NULL,
    AlertModeByCH       SMALLINT NOT NULL,
    KeyToneOn           SMALLINT NOT NULL DEFAULT '0',
    CradleType          SMALLINT NOT NULL DEFAULT '0',
    PowerControlMode    SMALLINT NOT NULL DEFAULT '0',
	SpkOnOffOnceDuringACall SMALLINT NOT NULL DEFAULT '1',
    BTHeadsetNotification  SMALLINT NOT NULL DEFAULT '1',
    DutyStatusACK  SMALLINT NOT NULL DEFAULT '0',
    OORStatusACK  SMALLINT NOT NULL DEFAULT '0',
    PowerOffACK  SMALLINT NOT NULL DEFAULT '0',
    TGHoldFunc 	SMALLINT NOT NULL DEFAULT '0',
    TGRemvFunc SMALLINT NOT NULL DEFAULT '0',
    UnholdAlert SMALLINT NOT NULL DEFAULT '0',
    SignalLevel INTEGER NOT NULL DEFAULT '0'
);
CREATE TABLE tabFunctionsParam (
    TGHoldTimeout INTEGER NOT NULL DEFAULT '0',
    StunChID INTEGER NOT NULL DEFAULT '0'
);
CREATE TABLE [tabGPRSAckSetting] ([FuncOption] INTEGER NOT NULL DEFAULT '0', [IP] CHAR (40) NOT NULL DEFAULT '203.74.115.194', [Port] CHAR (5) NOT NULL DEFAULT '8084', [ServerAPN] CHAR (32) NOT NULL DEFAULT 'INTERNET', [NameAPN] CHAR (32) NOT NULL DEFAULT 'IP', [UserNameAPN] CHAR (32), [PasswordAPN] CHAR (32));
CREATE TABLE [tabGPRSReceiveSetting] ([FuncOption] INTEGER NOT NULL DEFAULT '0', [IP1] CHAR (40) NOT NULL DEFAULT '203.74.115.194', [Port1] CHAR (5) NOT NULL DEFAULT '8085', [IP2] CHAR (40) NOT NULL DEFAULT '59.124.64.99', [Port2] BOOLEAN (5) NOT NULL DEFAULT '9103', [ConnectMsgOOR] CHAR (40) NOT NULL DEFAULT 'GPRS Connect', [DisconnectMsgOOR] CHAR (40) NOT NULL DEFAULT 'GPRS Disconnect', [SignalCheckTime] INTEGER NOT NULL DEFAULT '3', [AliveMsgInterval] INTEGER NOT NULL DEFAULT '30');
CREATE TABLE tabGPSDisplaySetting (
    GPSShowType  INTEGER NOT NULL
);
CREATE TABLE tabGPSEngSettings
(
    SetingID INTEGER NOT NULL,
    MonitorInterval  INTEGER,
    LostSignalTime INTEGER,
    SaveGpsInterval  INTEGER,
    SaveGpsDistance  INTEGER,
    ResetInterval  INTEGER,
    PRIMARY KEY(SetingID)
);
CREATE TABLE tabGPSInformSettings(
  IntervalForInform INTEGER,
  listAllFlag INTEGER
);
CREATE TABLE tabGPSInformTarget(
  ZoneId INTEGER,
  KnobId INTEGER,
  ToContactId INTEGER,
  OnOff INTEGER
);
CREATE TABLE tabGPSLocationDisplaySetting (
    MemberListCallOption  INTEGER NOT NULL    DEFAULT 0
);
CREATE TABLE [tabGSMBaseSetting] ([IsUseGSM] INTEGER NOT NULL DEFAULT '1', [PIN] CHAR (9) NOT NULL DEFAULT '0000', [Identity] INTEGER NOT NULL DEFAULT '0', [ManualTimeOut] INTEGER NOT NULL DEFAULT '5', [DataFormat] CHAR (32) NOT NULL DEFAULT '#I#S#M', [AutoOffTime] INTEGER NOT NULL DEFAULT '3', [AckSendPath] INTEGER NOT NULL DEFAULT '0', [PagerID] VARCHAR (7) NOT NULL DEFAULT '0000000', [IMEI] VARCHAR (15) NOT NULL DEFAULT '000000000000000', [IMSI] VARCHAR (15) NOT NULL DEFAULT '000000000000000', [GsmFwVersion] VARCHAR (50) NOT NULL DEFAULT '00000000000000000000', [phoneVolumeLevel] INTEGER DEFAULT '60', [phoneMicGainHandsetFront] INTEGER DEFAULT '10', [phoneMicGainHandsetDigital] INTEGER DEFAULT '8192', [retryCount] INTEGER DEFAULT '12', [retryInterval] INTEGER DEFAULT '5000',[heartbeat] INTEGER DEFAULT '60');
CREATE TABLE tabGSMCallContactList(
  id INTEGER,
  Name INTEGER,
  Number INTEGER,
  PRIMARY KEY(id)
);
CREATE TABLE tabGsmBusinessSetting(
   funId INTEGER NOT NULL,
   transmitType INTEGER ,
   dataType INTEGER ,
   priority INTEGER ,
   standbyTime INTEGER ,
   connectRetry INTEGER ,
   sendRetry INTEGER ,
   bizRetry INTEGER ,
   ackTime INTEGER ,
   respTime INTEGER ,
   waitTime INTEGER ,
   PRIMARY KEY(funId)
);
CREATE TABLE tabHardKeyActionOnKeypadLock (
    KeyCode    INTEGER NOT NULL,
    KeyName    VARCHAR,
    ActionType INTEGER NOT NULL
);
create table tabHdPhotoProfile (
SettingId integer,
quantization integer,
TileWidth integer,
TileHeight integer,
primary key (SettingId));
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
CREATE TABLE tabHomeSettings ( isFrequencyVisible SMALLINT NOT NULL , P25TSystemShowOption SMALLINT NOT NULL , P25TSystemName VARCHAR ( 256 ) , HomeStyle SMALLINT , NumberFormat SMALLINT , ScnningTime INTEGER , PromptShowTime INTEGER , PromptHideTime INTEGER , ScanFrqTime INTEGER , IsRfssSiteVisible BOOLEAN );
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
CREATE TABLE tabKeyinNumberCallSetting(
  calltype INTEGER,
  Keyin INTEGER
);
CREATE TABLE tabKeypadLockSetting (
    AutoKeypadLock            INTEGER NOT NULL,
    KeypadLockComboDetectTime INTEGER,
    PlayKeyDepressTime        INTEGER
);
CREATE TABLE tabLedSetting (
  SettingID INTEGER, Period INTEGER, Percent INTEGER, Cycles INTEGER, interval INTEGER, PRIMARY KEY(SettingID)
);
CREATE TABLE tabLightSensorAndBackLight(
  lightSensor INTEGER NOT NULL,
  backLight INTEGER NOT NULL,
  PRIMARY KEY(lightSensor)
);
CREATE TABLE tabLightSensorLevelToBrightness(
  lightSensorLevel INTEGER NOT NULL,
  Brightness INTEGER NOT NULL,
  PRIMARY KEY(lightSensorLevel)
);
create table tabLimitFunctionsByProduct(
    FuncID INTEGER NOT NULL,
    primary key (FuncID)
);
create table tabLimitFunctionsByUser(
    FuncID INTEGER NOT NULL,
    primary key (FuncID)
);
CREATE TABLE tabM1DeviceType(
  ContactID INTEGER NOT NULL,
  DeviceType INTEGER NOT NULL,
  PRIMARY KEY(ContactID)
);
CREATE TABLE tabM1TransitWaitTime(
  WaitTime  INTEGER NOT NULL,
  PRIMARY KEY(WaitTime)
);
CREATE TABLE tabMDCRepeaterSetting(
   SACID  INTEGER NOT NULL,
   MdcPretime  INTEGER NOT NULL,
   MdcMsgId  INTEGER NOT NULL,
   MdcAutoRepeaterTxType  SMALLINT NOT NULL,
   PRIMARY KEY (SACID)
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
create table tabManDownSettings (
id smallint,
TlitSwitch smallint,
TlitLevel smallint,
TlitDuration smallint,
StillSwitch smallint,
StillDuration smallint,
FreeFallSwitch smallint,
FreeFallDuration smallint,
TlitPreAlertTime smallint,
StillPreAlertTime smallint,
FreeFallPreAlertTime smallint,
primary key (id));
CREATE TABLE tabManufactureInfo (
    ProductSNO VARCHAR(30) NULL,
    ManufacturedDate  VARCHAR(30) NULL,
    SWVer   VARCHAR(20) NULL,
    HWVer   VARCHAR(20) NULL,
    FactVer VARCHAR(20) NULL,
    RFFreqBandFlag INTEGER NOT NULL
);
CREATE TABLE tabMdc1200Signaling(
    SigID  INTEGER NOT NULL,
    BasicTxPreTime  SMALLINT default 1 NOT NULL,
    PmbBitSync INTEGER default 24 NOT NULL,
    PTTIDType   SMALLINT default 0 NOT NULL,
    InbnMsgRxEnable  SMALLINT default 0 NOT NULL,
    SignallingActivityTimeout  SMALLINT default 0 NOT NULL,
    ACKWaitTimeout    INTEGER NOT NULL,
    PRIMARY KEY(SigID)
);
CREATE TABLE tabMemory (
    funId        INTEGER     PRIMARY KEY
                             CHECK ( funId > 0 ),
    memType      INTEGER     NOT NULL
                             CHECK ( memType BETWEEN 0 AND 1 )
                             DEFAULT '0',
    maxMem       INTEGER     NOT NULL
                             CHECK ( maxMem > 0 ),
    overflowType INTEGER     NOT NULL
                             CHECK ( overflowType BETWEEN 0 AND 1 )
                             DEFAULT '1',
    hasAlert     INTEGER     NOT NULL
                             CHECK ( hasAlert BETWEEN 0 AND 1 )
                             DEFAULT '0',
    alertLv1     INTEGER     NOT NULL
                             CHECK ( alertLv1 > 0 ),
    alertLv2     INTEGER     NOT NULL
                             CHECK ( alertLv2 > 0 ),
    remark       CHAR( 64 )
);
CREATE TABLE tabMenuLayout (
    PageNo   INT            NOT NULL,
    Position INT            NOT NULL,
    ItemKey  VARCHAR( 10 )  NOT NULL
);
CREATE TABLE [tabMenuListLayout] ([Position] INT NOT NULL, [ItemKey] VARCHAR (10) NOT NULL);
CREATE TABLE [tabMenuListUISpec] ([row] INT NOT NULL, [fontsize]  INT NOT NULL);
CREATE TABLE tabMenuOption(
    RowCount INT DEFAULT 3,
    ColCount INT DEFAULT 3,
    DefaultHilite INT DEFAULT 4,
    Special VARCHAR(64)
);
CREATE TABLE [tabMenuUIType] ([type] INT NOT NULL);
create table tabMonitorModeSettings(
id integer,
Squechpercent integer,
primary key (id));
CREATE TABLE tabMorseAudioToneDetail(
                   ForceToTone INTEGER,
                   DitFrequency INTEGER,
                   DitDur INTEGER,
                   DahFrequency INTEGER,
                   DahDur INTEGER,
                   LeFrequency INTEGER,
                   LeDur INTEGER,
                   WeFrequency INTEGER,
                   WeDur INTEGER
);
CREATE TABLE tabMyContact(
    ID  INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
create table tabOTAPassWord(
    password varchar(6)
);
CREATE TABLE tabOtaMenuList(
  OTAMenuSerial INTEGER,
  OTASettingId INTEGER,
  Enable INTEGER,
  PRIMARY KEY(OTAMenuSerial)
);
CREATE TABLE tabOtaSettings(
    ControlZoneId  INTEGER NOT NULL,
    ControlknobId    INTEGER NOT NULL,
    ControlChannel INTEGER NOT NULL
);
CREATE TABLE tabOutRangeSetting (
    outRangeCheckTimer INTEGER ,
    inRangeRssi        INTEGER ,
    inRangeBER 		     INTEGER ,
    outRangeCheckCount INTEGER ,
    openRfTimer INTEGER default 30 NOT NULL,
    closeRfTimer INTEGER default 30 NOT NULL,
    AlertInterval INTEGER default 30 NOT NULL,
    AlertTimes    INTEGER default 30 NOT NULL,
    inRangeCheckTime INTEGER default 5 NOT NULL,
	InOutRangeAlert INTEGER default 1 NOT NULL,
	AlertTimeout INTEGER default 900 NOT NULL
);
CREATE TABLE tabOutputSettings(
    SyncOnOff INTEGER,
    SyncWay INTEGER,
    TargetId INTEGER,
    SyncResolutionX INTEGER,
    SyncResolutionY INTEGER,
    SaveOnOff INTEGER,
    SyncFrameRates INTEGER
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
CREATE TABLE tabP25RepeaterSetting(
   SACID  INTEGER NOT NULL,
   TxNAC  INTEGER NOT NULL,
   RxNAC  INTEGER NOT NULL,
   PRIMARY KEY (SACID)
);
CREATE TABLE tabP25Signaling(
    SigID  INTEGER NOT NULL,
    TXPreTime INTEGER default 10,
    NoSigActTimeout  INTEGER NOT NULL,
    ACKWaitTimeout    INTEGER NOT NULL,
    RxNAC INTEGER default 0,
    MFID        INTEGER default 0,
    AlgId    SMALLINT NOT NULL,
    MinShortDigitalToneDuration  SMALLINT NOT NULL,
    MinLongDigitalToneDuration  SMALLINT NOT NULL,
    MinToneAddrGap  SMALLINT NOT NULL,
    EncryptedCallFlag SMALLINT NOT NULL,
    ShortDigitalToneATxDuration SMALLINT default 1000 NOT NULL,
    ShortDigitalToneBTxDuration  SMALLINT default 2000 NOT NULL,
    LongDigitalToneBTxDuration  SMALLINT default 5000 NOT NULL,
    TwoDigitalToneAddrTxGap  SMALLINT default 1500 NOT NULL,
    PRIMARY KEY(SigID)
);
CREATE TABLE tabP25TQCDelayRecordParm (
    ChannelID      INTEGER        NOT NULL,
    RecvAddress    VARCHAR( 30 )  NOT NULL,
    ReGroupAddress VARCHAR( 30 )  NOT NULL,
    DelayNTimeSec  INTEGER        NOT NULL,
    DelayMTimeSec  INTEGER        NOT NULL
);
CREATE TABLE tabP25TSetting
(
    ChannelID INTEGER   NOT NULL,
    MSELCallSamplingFlag SMALLINT NOT NULL,
    MaxMSELSamplingTime SMALLINT NOT NULL,
    TwoToneSubGroupOn SMALLINT NOT NULL,
    MinShortDigitalToneDuration  SMALLINT NOT NULL,
    MinLongDigitalToneDuration  SMALLINT NOT NULL,
    MinToneAddrGap  SMALLINT NOT NULL,
    OTAToneFlag   SMALLINT NOT NULL,
    ShortDigitalToneATxDuration SMALLINT NOT NULL,
    ShortDigitalToneBTxDuration  SMALLINT NOT NULL,
    LongDigitalToneBTxDuration  SMALLINT NOT NULL,
    TwoDigitalToneAddrTxGap  SMALLINT NOT NULL,
    EncryptedCallFlag SMALLINT NOT NULL,
    PRIMARY KEY(ChannelID)
);
CREATE TABLE tabP25TrunkingControlChannels(
    SigID INTEGER NOT NULL,
    RxChFreq  INTEGER NOT NULL ,
    SeqNo     INTEGER NOT NULL ,
    RfssID    INTEGER NOT NULL ,
    SiteID    INTEGER NOT NULL ,
    PRIMARY KEY(SigID,RxChFreq)
);
CREATE TABLE tabP25TrunkingSignaling(
    SigID INTEGER NOT NULL,
    ACKWaitTimeout     INTEGER NOT NULL,
    RfssRspWaitTimeout INTEGER NOT NULL,
    MFID               INTEGER default 0,
    AlgId              SMALLINT NOT NULL,
    SystemID           INTEGER NOT NULL,
    WacnID             INTEGER NOT NULL,
    WacnSystemIDCheckSkipFlag INTEGER NOT NULL,
    FullSpectrumScanFlag INTEGER NOT NULL,
    FullSpectrumScanTimer INTEGER NOT NULL,
    FullSpectrumScanRanges INTEGER NOT NULL,
    StartFreqOfRange1 INTEGER NOT NULL,
    EndFreqOfRange1 INTEGER NOT NULL,
    StepSizeOfRange1 INTEGER NOT NULL,
    StartFreqOfRange2 INTEGER,
    EndFreqOfRange2 INTEGER,
    StepSizeOfRange2 INTEGER,
    StartFreqOfRange3 INTEGER,
    EndFreqOfRange3 INTEGER,
    StepSizeOfRange3 INTEGER,
    SiteTrunkingFlag SMALLINT NOT NULL,
    OutOfRangeErrorRate INTEGER NOT NULL,
    PRIMARY KEY(SigID)
);
CREATE TABLE tabP25TrunkingSites(
    SigID INTEGER NOT NULL,
    RfssID    INTEGER NOT NULL ,
    SiteID    INTEGER NOT NULL,
    SiteName   varchar(16) NOT NULL,
    SeqNo     INTEGER default 0 NOT NULL,
    PRIMARY KEY(SigID,RfssID,SiteID)
);
CREATE TABLE tabPINPasswordSetting(
    ID                INTEGER NOT NULL,
    BootingLockEnable INTEGER,
    KeypadLockEnable  INTEGER,
    AdminPINPwd       VARCHAR(64),
    DefaultPINPwd     VARCHAR(64),
    UserPINPwd        VARCHAR(64),
    MaxErrorCount     INTEGER,
    ErrorDisableTime  INTEGER,
    primary key (ID)
);
CREATE TABLE tabPagerMsgBoxSetting(
    Duration INTEGER,
    LogCount INTEGER
);
CREATE TABLE tabPagerMsgBoxSetting_New(
    Type INTEGER,
    MaxNum INTEGER,
    MaxLockNum INTEGER,
    PRIMARY KEY(Type)
);
create table tabPowerSavingRxOff(DefaultTime integer, TwoTone integer, DMR integer, P25C integer, P25T integer);
CREATE TABLE tabPowerSavingSetting (
    ShutdownTimeAfterSuspend INTEGER
);
CREATE TABLE tabR2RModeSetting(
    SettingID INTEGER,
    Mode INTEGER Default 1 NOT NULL,
    PRIMARY KEY(SettingID)
);
create table tabRFPreformance (
    TypeID integer,
Param integer,
primary key(TypeID)
);
CREATE TABLE tabRadioBaseSetting (
    UserCompany VARCHAR(128) default 'U3' NOT NULL,
    UserName VARCHAR(64) default 'U3' NOT NULL,
    Password VARCHAR(20) default '000000' NOT NULL,
    RadioNo VARCHAR(32)
);
CREATE TABLE tabRadioGroup (
    PrivateId INTEGER,
    GroupId INTEGER,
PRIMARY KEY (PrivateId, GroupId)
);
CREATE TABLE tabReceivingMode(
    ZoneID INTEGER NOT NULL,
    KnobID INTEGER NOT NULL,
    SupportedMode SMALLINT NOT NULL,
    CurrentMode SMALLINT NOT NULL,
    DualCHScanSettingID SMALLINT NOT NULL DEFAULT 0,
    MultiCHScanSettingID SMALLINT NOT NULL DEFAULT 0,
    FreeScanSettingID SMALLINT NOT NULL DEFAULT 0,
    CreateFlag INTEGER default 1 NOT NULL,
    Name      VARCHAR(16) NOT NULL,
    CtCdRequest smallinit NOT NULL,
    ColorCodeRequest smallinit NOT NULL,
    NACRequest smallinit NOT NULL,
    PRIMARY KEY(ZoneID,KnobID)
);
CREATE TABLE tabRecordSettings(
    MaxLenForAll  INTEGER NOT NULL,
    MaxLenForEach INTEGER NOT NULL,
    MaxRecCount INTEGER NOT NULL,
    StoredPath  VARCHAR(50) NOT NULL
);
CREATE TABLE tabRecvBackgroundColorSetting (
	FlashOnTime      SMALLINT NOT NULL DEFAULT '1',
	FlashOffTime     SMALLINT NOT NULL DEFAULT '1',
	FlashDuration    SMALLINT NOT NULL DEFAULT '0',
	IsVoiceFlash     SMALLINT NOT NULL DEFAULT '0',
	IsTextFlash      SMALLINT NOT NULL DEFAULT '0',
	IsImageFlash     SMALLINT NOT NULL DEFAULT '0'
);
CREATE TABLE tabRecvBehaviorByContactID (
    ChId INTEGER NOT NULL,
    RadioID INTEGER NOT NULL,
    Priority SMALLINT NOT NULL,
    Duty SMALLINT NOT NULL,
    AlertMode SMALLINT NOT NULL,
    VoiceStorage SMALLINT NOT NULL,
    SigType INTEGER  NOT NULL,
    VoiceRecvAck INTEGER,
    VoiceReadAck INTEGER,
    VoiceManualAck INTEGER,
    TextRecvAck INTEGER,
    TextReadAck INTEGER,
    TextManualAck INTEGER,
    ImageRecvAck INTEGER,
    ImageReadAck INTEGER,
    ImageManualAck INTEGER,
    PRIMARY KEY(ChId,RadioID,SigType)
);
CREATE TABLE tabRecvGroupList(
    ZoneKnobID INTEGER NOT NULL,
    SigType INTEGER  NOT NULL,
    SigID INTEGER  NOT NULL,
    GroupAddr VARCHAR(30),
    GroupId INTEGER,
    SeqNo INTEGER,
    PRIMARY KEY(ZoneKnobID,SigType,SigID,GroupId)
);
create table tabRecvImageForShowSetting(
  good   integer not null,
  normal integer not null,
  poor   integer not null
);
CREATE TABLE tabRecvSaList(
    ZoneKnobID INTEGER NOT NULL,
    SigType INTEGER  NOT NULL,
    SaType  INTEGER default 0 NOT NULL,
    ctFreq INTEGER,
    ctSte INTEGER,
    cdCodeWord INTEGER,
    cdSte INTEGER,
    cdModSense INTEGER
);
create table tabRecvToneForContacts(
 id integer,
 enable smallint,
 toneID integer,
 alias VARCHAR(128),
 sourceType smallint,
 sourceID smallint,
 type smallint,
 interval integer,
 maxTime integer,
 primary key (id)
);
CREATE TABLE tabRecvVoiceCallRecordByChannel(
   ZoneID INTEGER NOT NULL,
   KnobID INTEGER NOT NULL,
   AllowRecord SMALLINT NOT NULL,
   PRIMARY KEY(ZoneID,KnobID)
);
CREATE TABLE tabRecvVoiceCallRecordByContactID(
   DestContactID INTEGER NOT NULL,
   AllowRecord SMALLINT NOT NULL,
   PRIMARY KEY(DestContactID)
);
CREATE TABLE [tabReminderAlertSetting] ([option] INT PRIMARY KEY NOT NULL DEFAULT '0', [ReminderAlertInterval] INTEGER NOT NULL DEFAULT '30
', [ReminderAlertTimeout] INTEGER NOT NULL DEFAULT '30', [ReminderAlertDuration] INTEGER NOT NULL DEFAULT '5');
CREATE TABLE tabRemoteMonitorSettings (SettingID INTEGER, MonitorTime INTEGER, AutoRecord INTEGER, IsSilence INTEGER, RxDurMultiplier INTEGER, TxDurMultiplier INTEGER, GlobalMultiplierEnable INTEGER, CallBackType INTEGER);
CREATE TABLE tabRepeaterSettingOfChan(
    ChannelID  INTEGER NOT NULL,
    SignalingType  INTEGER NOT NULL,
    AccessCodeType INTEGER NOT NULL,
    RACID   INTEGER default 0 NOT NULL,
    TxSaType  INTEGER default 0 NOT NULL,
    TxCtFreq  INTEGER default 0 NOT NULL,
    TxCtSte  INTEGER default 0 NOT NULL,
    TxCdCodeWord  INTEGER default 0 NOT NULL,
    TxCdSte  INTEGER default 0 NOT NULL,
    TxCdModSense  INTEGER default 0 NOT NULL,
    RxSaType  INTEGER default 0 NOT NULL,
    RxCtFreq  INTEGER default 0 NOT NULL,
    RxCdCodeWord  INTEGER default 0 NOT NULL,
    RxCdModSense  INTEGER default 0 NOT NULL,
    PRIMARY KEY (ChannelID,SignalingType)
);
CREATE TABLE tabResetModeSetting(
    ID INTEGER NOT NULL,
    ResetModeClass SMALLINT DEFAULT 0 NOT NULL,
    DelayNTime SMALLINT DEFAULT 30 NOT NULL,
    EnableCarrierOffTime SMALLINT DEFAULT 0,
    CarrierOffDelayTime SMALLINT DEFAULT 30,
    EnableTgidStayTime SMALLINT DEFAULT 0,
    TgidStayTime  SMALLINT DEFAULT 30
);
CREATE TABLE tabRetryDonwloadSettings (
    OORRetryDownloadInterval INTEGER NOT NULL,
    ScanRetryDownloadInterval INTEGER NOT NULL,
    TrafficRetryDownloadInterval INTEGER NOT NULL,
    NoDataRetryInterval INTEGER NOT NULL,
    TimeCorrectionTimeout INTEGER NOT NULL
);
CREATE TABLE tabRetryDownloadInfo (
    lastSubscriptionTime TIMESTAMP NOT NULL,
    subscriptionZoneKnobId INTEGER NOT NULL,
    subscriptionTime TIMESTAMP NOT NULL DEFAULT 0,
    rfssId INTEGER NOT NULL,
    siteId INTEGER NOT NULL,
    tgIdList VARCHAR(1024) NOT NULL,
    timetoPagerStation INTEGER NOT NULL,
    timeCorrectionTime TIMESTAMP NOT NULL DEFAULT 0,
    isDownload INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE tabRevertListGroup (
    ID            INTEGER        NOT NULL,
    RecvAddress   VARCHAR( 30 )  NOT NULL,
    RevertAddress VARCHAR( 30 )  NOT NULL,
    SeqNo1        INTEGER        NOT NULL,
    SeqNo2        INTEGER        NOT NULL
);
CREATE TABLE [tabRingModeSetting] (
    [id] INTEGER PRIMARY KEY UNIQUE,
    [isOn] INTEGER NOT NULL DEFAULT '1',
    [mode] INTEGER NOT NULL DEFAULT '0',
    [time] INTEGER NOT NULL DEFAULT '2',
    [IntervalTime] INTEGER NOT NULL DEFAULT '1',
    [IsVibInterval] INTEGER NOT NULL DEFAULT '0',
    [VibIntervalOnTime] INTEGER NOT NULL DEFAULT '1',
    [VibIntervalOffTime] INTEGER NOT NULL DEFAULT '1',
    [isVoiceStartStopRing] INTEGER NOT NULL DEFAULT '0');
create table tabRssiRange(
   Normal INTEGER NOT NULL,
   Low    INTEGER NOT NULL,
   Weak   INTEGER NOT NULL,
   Lost   INTEGER NOT NULL
);
CREATE TABLE [tabSMSSetting] ([Phone] CHAR (20) NOT NULL DEFAULT '+000000000000', [EncodeFormat] INTEGER NOT NULL DEFAULT '1');
CREATE TABLE tabSameFreqChannelList(
    ZoneKnobID INTEGER NOT NULL,
    SameFreqZoneKnobIDList VARCHAR(1024),
    PRIMARY KEY(ZoneKnobID)
);
CREATE TABLE tabScanListSettings(
    ScanListSettingID  INTEGER NOT NULL,
    PriorityScanType  SMALLINT NOT NULL,
    SamplingFlag  SMALLINT NOT NULL,
    PriorityZoneKnobID  SMALLINT NOT NULL,
    ScanList  VARCHAR(1024) NOT NULL,
    RecvAllFlag  SMALLINT NOT NULL,
    StayTime INTEGER NOT NULL,
    CallBackTime INTEGER NOT NULL,
    RxPauseBetweenScanPaths INTEGER NOT NULL DEFAULT 0,
	PriorityIntervalTime INTEGER NOT NULL DEFAULT 5,
    PRIMARY KEY(ScanListSettingID)
);
CREATE TABLE tabSingleToneRepeaterSetting(
   SACID  INTEGER NOT NULL,
   SingleToneFreq  INTEGER NOT NULL,
   ToneDuration  INTEGER NOT NULL,
   PRIMARY KEY (SACID)
);
CREATE TABLE tabSpecialKeyActionTime(
    KeyCode    INTEGER NOT NULL,
    KeyName    VARCHAR,
    ActionTime INTEGER NOT NULL,
    primary key (KeyCode)
);
CREATE TABLE tabSpecialMenuLayout (
    ID      INT            PRIMARY KEY
                           NOT NULL
                           UNIQUE,
    ItemKey VARCHAR( 10 )  NOT NULL
                           UNIQUE
);
CREATE TABLE tabSubgroupWithMultiSubAddr(
    SubGroupNo INTEGER NOT NULL,
    SubGroupContactId INTEGER NOT NULL,
    PRIMARY KEY(SubgroupNo, SubGroupContactId)
);
CREATE TABLE tabSystemAlias (
    SigType     SMALLINT,
    SigId       SMALLINT,
    SystemAlias VARCHAR(11),
    PRIMARY KEY (SigType,SigId)
);
CREATE TABLE tabTextMessageSetting(
    SettingId SMALLINT,
    MaxTextNoteLen INTEGER,
    MergeMaxDuration INTEGER,
    SimilarPercent INTEGER,
    PRIMARY KEY(SettingId)
);
CREATE TABLE tabTimerSetting(
id  SMALLINT ,
alarmTime INTEGER,
alarmType SMALLINT,
type SMALLINT,
weekBit INTEGER,
PRIMARY KEY(id)
);
CREATE TABLE tabTodaySettings(
    isAlertToneOn INTEGER NOT NULL,
    AlertToneInterval INTEGER NOT NULL
);
CREATE TABLE tabTone(
   id    INTEGER NOT NULL,
   alias VARCHAR(30),
   type  SMALLINT NOT NULL,
   sourceID NTEGER NOT NULL,
   PRIMARY KEY(id)
);
create table tabToneFileDetail(
sourceID integer,
type integer,
file VARCHAR(128),
primary key (sourceID));
create table tabToneFreqDetail(
sourceID integer,
indexId integer,
freqA integer,
freqB integer,
duration integer,
primary key (sourceID, indexId)
);
CREATE TABLE tabTransmitPriority (
    CommandID INTEGER NOT NULL,
    PriorityCode INTEGER default 4 NOT NULL,
    StandbyTime INTEGER default 0 NOT NULL,
    PRIMARY KEY(CommandID)
);
create table tabTransmitSchedulerSettings(
  SendingTimeSlice INTEGER NOT NULL,
  WaitTimeForNextSending INTEGER NOT NULL
);
CREATE TABLE tabTwoToneSignaling(
    SigID  INTEGER NOT NULL,
    PreTime SMALLINT default 10 NOT NULL,
    Tone1TxDuration  INTEGER default 200 NOT NULL,
    Gap1TxDuration  SMALLINT default 0 NOT NULL,
    Tone2TxDuration  INTEGER default 200 NOT NULL,
    Gap2TxDuration  SMALLINT default 0 NOT NULL,
    LongToneDuration INTEGER default 200 NOT NULL,
    LongToneTxGap  SMALLINT default 0 NOT NULL,
    LongToneRxTmout  INTEGER default 3000 NOT NULL,
    SignallingActivityTimeout  INTEGER default 10 NOT NULL,
    RxTimeout   INTEGER default 3000 NOT NULL,
    OtaTonePlayFlag   SMALLINT default 1 NOT NULL,
    MinToneDuration   SMALLINT default 400 NOT NULL,
    NSigPwrOnThreshold INTEGER default 3162278 NOT NULL,
    NSigPwrOffThreshold INTEGER default 1778279 NOT NULL,
    WSigPwrOnThreshold INTEGER default 3162278 NOT NULL,
    WSigPwrOffThreshold INTEGER default 3162278 NOT NULL,
    PRIMARY KEY(SigID)
);
CREATE TABLE tabUdc2400Signaling(
    SigID  INTEGER NOT NULL,
    BasicTxPreTime  SMALLINT default 1 NOT NULL,
    PmbBitSync INTEGER default 24 NOT NULL,
    PTTIDType   SMALLINT default 0 NOT NULL,
    InbnMsgRxEnable  SMALLINT default 0 NOT NULL,
    SignallingActivityTimeout  SMALLINT default 0 NOT NULL,
    ACKWaitTimeout    INTEGER NOT NULL,
    PRIMARY KEY(SigID)
);
CREATE TABLE tabUiNumberFormat(
		CallIdShowType SMALLINT NOT NULL,
		SysIdShowType  SMALLINT NOT NULL,
		FrequencyUnit  SMALLINT NOT NULL
);
CREATE TABLE tabUnreadAlertSettings(
    AlertType INTEGER NOT NULL,
    AlertTimes INTEGER NOT NULL,
    AlertToneInterval INTEGER NOT NULL
);
CREATE TABLE tabUseDefineCHDetail (
    ChannelID     INTEGER,
    DateTime      VARCHAR( 30 ),
    SignalingType SMALLINT,
    RXFrequency   INTEGER,
    RXBandWidth   INTEGER,
    FromAddr      VARCHAR( 100 ),
    ToAddr        VARCHAR( 100 ),
    NAC           INTEGER,
    colorCode     INTEGER,
    SASubAudio    VARCHAR( 100 ),
    BER           INTEGER,
    encryption    INTEGER,
    emgService    SMALLINT,
    priority      INTEGER,
    signalLevel   INTEGER
);
CREATE TABLE tabVibrate(
   VibPatternID    INTEGER NOT NULL,
   VibID INTEGER NOT NULL,
   PRIMARY KEY(VibPatternID)
);
CREATE TABLE tabVibrateDetail(
   VibID    INTEGER NOT NULL,
   IndexId INTEGER NOT NULL,
   enable INTEGER NOT NULL,
   Duration INTEGER NOT NULL,
   PRIMARY KEY(VibID,IndexId)
);
CREATE TABLE tabVoiceCallSettings (
    ID  INTEGER NOT NULL,
    DefaultCallID  INTEGER  NOT NULL,
    HangTimeout  INTEGER NOT NULL,
    PenaltyTime  INTEGER default 0 NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabVoiceMemoSettings (
    ID INTEGER NOT NULL DEFAULT 1,
    AlertToneDur INTEGER NOT NULL DEFAULT 15,
    PlayOption INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(ID)
);
CREATE TABLE tabVoiceQualitySetting (
    voiceQualityCheckTimer 			INTEGER ,
    voiceNotClearRssi        		INTEGER ,
    voiceNotClearBER 		     	INTEGER ,
    voiceNotClearCheckCount 		INTEGER ,
    AutoRegetVr 					INTEGER
);
CREATE TABLE tabVolumeLevelSetting(
    MinValue INTEGER,
    MaxValue INTEGER,
    Level INTEGER,
    PRIMARY KEY(MinValue,Level)
);
CREATE TABLE tabWatchDog (
    open INTEGER,
    breed INTEGER,
    normalFeedInterval INTEGER,
    powerSavingFeedInterval INTEGER
);
CREATE TABLE tabZone(
    ID  INTEGER NOT NULL,
    Name VARCHAR(30) NOT NULL,
    ModifyByDevice SMALLINT NOT NULL,
    priorityToneID INTEGER NOT NULL,
    PRIMARY KEY(ID)
);
CREATE TABLE tabZoneChannelList(
   ZoneID INTEGER NOT NULL,
   KnobIndex SMALLINT NOT NULL,
   ChannelID INTEGER NOT NULL,
   CreateFlag INTEGER default 1 NOT NULL,
   PRIMARY KEY(ZoneID,KnobIndex)
);
CREATE TABLE tabZoneDetailSettings(
  isTGIDCodeVisible SMALLINT NOT NULL
);
CREATE UNIQUE INDEX [idx_tabDefaultMenuLayout] ON [tabMenuLayout] ( [PageNo] ASC , [Position] ASC );
