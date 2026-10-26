CREATE TABLE tabPwdControlGlobal (
		ResetTimeDur INTEGER,
		TotalErrorCount INTEGER
);
CREATE TABLE tabPwdControlUser (
    Type INTEGER,
		ResetTimeStamp VARCHAR(25),
		CurrentErrorInputCount INTEGER DEFAULT 0
);
