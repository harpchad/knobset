CREATE TABLE TabConnectionType (
		ConnectionType INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE TabEthernetSetting (
		IP VARCHAR(20),
		Port INTEGER
);
CREATE TABLE TabUSBSetting (
		IP VARCHAR(20),
		Port INTEGER
);
CREATE TABLE tabLoginPPS (LoginUserId INT, LoginPpsType INT, LoginUserName VARCHAR(64), LoginUserPW VARCHAR(32), SysLoginUserCreateDateTime VARCHAR(25), SysLastestLoginDateTime VARCHAR(25));
