CREATE TABLE tabEditPermission(
    PermissionID INTEGER NOT NULL,
    SeqNo INTEGER NOT NULL,
    ParamName VARCHAR(32) NOT NULL,
    Enable INTEGER NOT NULL,
		Permission INTEGER NOT NULL,
	PRIMARY KEY(PermissionID)
);
CREATE TABLE tabGroup_New(
    GroupID VARCHAR(64),
    GroupName VARCHAR(14),
    GroupNo VARCHAR(18),
    Note VARCHAR(240),
    RecordFlag INTEGER DEFAULT 1,
    Type INTEGER DEFAULT 0,
    Relay INTEGER DEFAULT 0,
    Priority INTEGER DEFAULT 0,
    DutyStatus INTEGER DEFAULT 1,
    AlertMode INTEGER DEFAULT 1,
    AlertToneID VARCHAR(64) DEFAULT '9999',
    BackgroundColor SMALLINT NOT NULL DEFAULT '0',
    EncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    KeyStrapOption INTEGER NOT NULL DEFAULT 1,
		P25TAddr VARCHAR (30)
);
CREATE TABLE tabMember_New(
    MemberID VARCHAR(64),
    MemberName VARCHAR(16),
    MemberType INTEGER,
    RadioNo INTEGER,
    RadioType INTEGER,
    Note VARCHAR(240),
    RecordFlag INTEGER DEFAULT 1,
    Relay INTEGER DEFAULT 0,
    Priority INTEGER DEFAULT 0,
    DutyStatus INTEGER DEFAULT 0,
    AlertMode INTEGER DEFAULT 1,
    AlertToneID VARCHAR(64) DEFAULT '9999',
    BackgroundColor SMALLINT NOT NULL DEFAULT '0',
    EncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    KeyStrapOption INTEGER NOT NULL DEFAULT 1,
		P25TAddr VARCHAR (30)
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
    ShortDigitalToneATxDuration  SMALLINT NOT NULL,
    ShortDigitalToneBTxDuration  SMALLINT NOT NULL,
    LongDigitalToneTxDuration  SMALLINT NOT NULL,
    TxToneAddrGap  SMALLINT NOT NULL
);
CREATE TABLE tabP25TrunkingControlCH(
  ID INTEGER NOT NULL,
  RxFreq INTEGER,
  RxBandWidth INTEGER,
  TxFreq INTEGER,
  TxBandWidth INTEGER,
  Note VARCHAR(128),
  Primary key(ID)
);
CREATE TABLE tabP25TrunkingSite
(
  ID INTEGER  NOT NULL,
  Name VARCHAR(14) NOT NULL,
  RFSSID INTEGER NOT NULL Default 1,
  SiteID INTEGER NOT NULL Default 1,
  Note VARCHAR(128)
);
CREATE TABLE tabP25TrunkingSiteControlCH(
  SiteID INTEGER NOT NULL,
  ControlChannelID INTEGER NOT NULL
);
CREATE TABLE tabP25TrunkingSystem
(
  ID  VARCHAR(64) NOT NULL,
  SystemCreateType INTEGER,
  SystemSource VARCHAR(256),
  Name VARCHAR(14),
  ConverageType INTEGER,
  WacnID  INTEGER NOT NULL,
  SystemID INTEGER NOT NULL,
  FullSpectrumScanFlag INTEGER NOT NULL,
  FullSpectrumScanRanges INTEGER NOT NULL,
  StartFreqOfRange1 INTEGER,
  EndFreqOfRange1 INTEGER,
  StepSizeOfRange1 INTEGER,
  StartFreqOfRange2 INTEGER,
  EndFreqOfRange2 INTEGER,
  StepSizeOfRange2 INTEGER,
  StartFreqOfRange3 INTEGER,
  EndFreqOfRange3 INTEGER,
  StepSizeOfRange3 INTEGER,
  SiteTrunkingFlag INTEGER NOT NULL DEFAULT 0,
  ErrorRateThreshold INTEGER NOT NULL DEFAULT 5,
  SUID  VARCHAR(32),
  Primary key(ID)
);
CREATE TABLE tabRFBandTypeInfo (
    ID INTEGER NOT NULL DEFAULT 0,
    Code VARCHAR(3),
    Name VARCHAR(16),
    MinFreq INTEGER NOT NULL DEFAULT 0,
    MaxFreq INTEGER NOT NULL DEFAULT 0,
    Comment VARCHAR(128)
);
CREATE TABLE tabSUID(
    MSN VARCHAR(32) NOT NULL,
    P25TSUID VARCHAR(32) NOT NULL
);
CREATE TABLE tabSubGroup(
    GroupID VARCHAR(64),
    GroupName VARCHAR(14),
    Note VARCHAR(240),
    RecordFlag INTEGER  DEFAULT 1,
    Relay INTEGER  DEFAULT 0,
    Priority INTEGER  DEFAULT 0,
    DutyStatus INTEGER  DEFAULT 1,
    AlertMode INTEGER DEFAULT 1,
    AlertToneID VARCHAR(64) DEFAULT '9999',
    BackgroundColor SMALLINT NOT NULL DEFAULT '0',
    EncryptionKeyID VARCHAR(64) DEFAULT '000000000000',
    KeyStrapOption INTEGER NOT NULL DEFAULT 1,
		P25TSub1Addr VARCHAR (30),
		P25TSub2Addr VARCHAR (30),
		P25TSub3Addr VARCHAR (30)
);
CREATE TABLE tabVersion(
    version VARCHAR(32) NOT NULL
);
