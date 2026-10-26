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
