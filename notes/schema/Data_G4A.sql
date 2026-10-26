CREATE TABLE tabAOrBFunction
(
    FunctionIDA INTEGER,
    FunctionIDB INTEGER
);
CREATE TABLE tabBusinessOption (
    ChannelLimit INT,
    ZoneLimit INT,
    ConvFreqLimit INT,
    TrunkingSystemLimit INT,
    TrunkingCtrlCHLimit INT,
    TrunkingSiteLimit INT,
    ProtocolLimit INT,
    GroupLimit INT,
	RadioLimit INT
);
CREATE TABLE tabCustomerSupport (
    ID INTEGER,
    Name VARCHAR(64),
    SupportInfo VARCHAR(128)
);
CREATE TABLE tabCustomerType (
    Code VARCHAR(3),
    Name VARCHAR(64)
);
CREATE TABLE tabDeviceModel(
    ModelID  INTEGER NOT NULL,
    ModelName VARCHAR(10) NOT NULL,
    Enable INTEGER NOT NULL,
    PRIMARY KEY(ModelID)
);
CREATE TABLE tabEnablePassiveWorkStationFunctionsBase
(
    PassiveFuncID INTEGER,
    HWBlock VARCHAR(16),
    FunctionID VARCHAR(64),
    Model VARCHAR(64)
);
CREATE TABLE tabFiveToneFreqSets(
    ID  INTEGER NOT NULL,
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
CREATE TABLE tabFiveToneTable(
    ID VARCHAR(64),
    FreqSetID VARCHAR(64),
	Name VARCHAR(16) NOT NULL,
    SrcIdTxFlag INTEGER,
    Link2TxType INTEGER,
    AckTxType INTEGER,
	RepeaterAccessTxType INTEGER NOT NULL,
    DigitNum SMALLINT  NOT NULL,
    ToneDigit1 SMALLINT,
    ToneDigit2 SMALLINT,
    ToneDigit3 SMALLINT,
    ToneDigit4 SMALLINT,
    ToneDigit5 SMALLINT,
    ToneDigit6 SMALLINT,
    ToneDigit7 SMALLINT,
    NormalDigitTxDur INTEGER,
    FirstRepeaterIdTxDuration INTEGER NOT NULL,
    FirstSrcIdTxDur INTEGER,
    FirstDestIdTxDur INTEGER,
    LinkrTxDur INTEGER,
    Tonelink1TxDuration INTEGER NOT NULL,
	Link2TxDur INTEGER,
    LinkaTxDur INTEGER,
    LeadOutTxDur INTEGER,
    NoSigActTimeout INTEGER,
    ACKWaitTimeout INTEGER,
    RxToneGap INTEGER,
    RxTimeout INTEGER,
    ACKWaitFlag INTEGER,
    PRIMARY KEY(ID)
);
CREATE TABLE tabFunctionForUser
(
    TypeForUI INTEGER,
    FunctionID INTEGER,
    Sequence INTEGER
);
CREATE TABLE tabFunctionSettingBase
(
    SettingID INTEGER NOT NULL,
    DefaultOrder INETEGER,
    Name VARCHAR(64),
    HWBlock VARCHAR(16),
    FunctionID VARCHAR(64),
    Model VARCHAR(64)
);
CREATE TABLE tabFunctionStatusBase
(
    SettingItemId SMALLINT NOT NULL PRIMARY KEY,
    DefaultOrder INETEGER,
    Name VARCHAR(64),
    ItemType SMALLINT NOT NULL,
    HasChild SMALLINT NOT NULL,
    BelongSettingItem SMALLINT DEFAULT NULL,
    HWBlock VARCHAR(16),
    FunctionID VARCHAR(64),
    Model VARCHAR(64)
);
CREATE TABLE tabFunctionTypeForUser
(
    TypeID INTEGER,
    TypeName VARCHAR(128)
);
CREATE TABLE tabHWCodeName (
    HWCode VARCHAR(3) NOT NULL,
    Name VARCHAR(2) NOT NULL
);
CREATE TABLE tabHardWareBlock(
    Code VARCHAR(3),
    Name VARCHAR(64)
);
CREATE TABLE tabImportOldDataConfig
(
    OldPPSVersion VARCHAR(64) NOT NULL,
    OldDataPath VARCHAR(64) NOT NULL
);
CREATE TABLE tabImportOldVoiceDataConfig
(
    OldDataPath VARCHAR(64) NOT NULL
);
CREATE TABLE tabLanguageCode (
    Code VARCHAR(16),
    Name VARCHAR(64)
);
CREATE TABLE tabMenuBase
(
    ItemKey VARCHAR(16) NOT NULL,
    DefaultOrder INETEGER,
    Icon VARCHAR(64),
    Name VARCHAR(64),
    HWBlock VARCHAR(16),
    FunctionID VARCHAR(64),
    Model VARCHAR(64)
);
CREATE TABLE tabModelNumberInfo(
    ID  INTEGER NOT NULL,
    ModelNumber VARCHAR(200) NOT NULL,
    DbFolderName VARCHAR(250) NOT NULL,
    Enable INTEGER NOT NULL,
	Engineer INTEGER,
    Customer INTEGER,
    PRIMARY KEY(ID)
);
CREATE TABLE tabNationCode
(
    Code VARCHAR(2),
    Name VARCHAR(120)
);
CREATE TABLE tabOtaMenuBase
(
    SettingID INTEGER NOT NULL,
    DefaultOrder INETEGER,
    Name VARCHAR(64),
    HWBlock VARCHAR(16),
    FunctionID VARCHAR(64),
    Model VARCHAR(64)
);
CREATE TABLE tabP25TDigitalToneAddr
(
    ToneTableType INTEGER NOT NULL,
    ToneIndex  INTEGER NOT NULL,
    minFreq VARCHAR(10) NOT NULL,
    maxFreq VARCHAR(10) NOT NULL,
    decodeFreq VARCHAR(10) NOT NULL,
    PRIMARY KEY ( ToneTableType, ToneIndex )
);
CREATE TABLE tabPartnerFunction
(
    FunctionIDA INTEGER,
    FunctionIDB INTEGER
);
CREATE TABLE tabProductCategory(
    SysCode VARCHAR(3),
    Code VARCHAR(3),
    Name VARCHAR(64)
);
CREATE TABLE tabProductSeries(
    SysCode VARCHAR(3),
    CateCode VARCHAR(3),
    Code VARCHAR(3),
    Name VARCHAR(64)
);
CREATE TABLE tabProductSystemCategory(
    Code VARCHAR(3),
    Name VARCHAR(64)
);
CREATE TABLE tabProtocolCode
(
    ProtocolCode VARCHAR(3) NOT NULL,
    ProtocolModeHex VARCHAR(8) NOT NULL
);
CREATE TABLE tabRFBandRanges (
    BandTypeID INTEGER NOT NULL,
    RxTx INTEGER NOT NULL,
    RangeNo INTEGER NOT NULL,
    MinFreq INTEGER NOT NULL,
    MaxFreq INTEGER NOT NULL
);
CREATE TABLE tabRFBandTypeInfo (
    ID INTEGER NOT NULL DEFAULT 0,
    Code VARCHAR(3),
    Name VARCHAR(16),
    MinFreq INTEGER NOT NULL DEFAULT 0,
    MaxFreq INTEGER NOT NULL DEFAULT 0,
    Comment VARCHAR(128)
);
CREATE TABLE tabRelyFunction
(
    SonFunctionID INTEGER,
    FatherFunctionID INTEGER
);
CREATE TABLE tabSWCodeName (
    SWCode VARCHAR(3) NOT NULL,
    Name VARCHAR(2) NOT NULL
);
CREATE TABLE tabSpecialMenuBase
(
    ItemKey VARCHAR(16) NOT NULL,
    DefaultOrder INETEGER,
    Icon VARCHAR(64),
    Name VARCHAR(64),
    HWBlock VARCHAR(16),
    FunctionID VARCHAR(64)
);
CREATE TABLE tabSupportU3DBVersion
(
    SupportedU3DBVersion VARCHAR(32)
);
CREATE TABLE tabTwoToneTable
(
    ID VARCHAR(64) NOT NULL,
    Name VARCHAR(16) NOT NULL,
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
    MinToneDuration SMALLINT default 400 NOT NULL,
    PRIMARY KEY ([ID])
);
CREATE TABLE tabTwoToneTableFreqSets
(
    ToneTableID VARCHAR(64) NOT NULL,
    ToneGroupID INTEGER NOT NULL,
    ToneFreq INTEGER NOT NULL
);
CREATE TABLE tabTwoToneTableGroup
(
    ToneTableID VARCHAR(64) NOT NULL,
    ToneGroupID INTEGER NOT NULL,
    ToneGroupName VARCHAR(32) NOT NULL
);
CREATE TABLE tabU4Function
(
    CustomerType VARCHAR(2),
    FunctionID INTEGER,
    FunctionName VARCHAR(128),
    TypeID INTEGER,
    ParentFuncID INTEGER,
    FunctionOnOff INTEGER
);
CREATE TABLE tabU4FunctionType
(
    TypeID INTEGER,
    TypeName VARCHAR(128)
);
