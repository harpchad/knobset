CREATE TABLE tabBoardVersionBSN (
    ID VARCHAR(64),
    Name VARCHAR(64),
    Version VARCHAR(32),
    BSN VARCHAR(64)
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
CREATE TABLE tabMfgProductMsn (
    ManuSerialNumber VARCHAR(64)
);
CREATE TABLE tabModel(
    Model VARCHAR(200)
);
