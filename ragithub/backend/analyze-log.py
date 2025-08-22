import json
import re
import sqlite3
import sys

months = {
    "Jan": 1,
    "Feb": 2,
    "Mar": 3,
    "Apr": 4,
    "May": 5,
    "Jun": 6,
    "Jul": 7,
    "Aug": 8,
    "Sep": 9,
    "Oct": 10,
    "Nov": 11,
    "Dec": 12,
}

def parse_log(log: str) -> list[dict]:
    result = []

    for line in log.split("\n"):
        if line.strip() == "":
            continue

        if (r := re.match(r"[a-zA-Z]+\,\s*(\d+)\s*([a-zA-Z]+)\s*(\d+)\s*(\d+)\:(\d+)\:(\d+)\s*\+\d+\s*\|\s*([^|]+)\|(.+)", line)) is None:
            print("Warning! no match:", line.__repr__())
            continue

        day, month, year, hour, minute, second, address, message = r.groups()
        day, month, year, hour, minute, second = int(day), months[month], int(year), int(hour), int(minute), int(second)

        if (r := re.match(r"\s*(GET|POST|PUT|DELETE)\s*([-a-zA-Z0-9/_.]+)\s*(\d{3})(.+)", message)) is not None:
            method, path, status, message = r.groups()
            status = int(status)

        else:
            method, path, status = None, None, None

        if (r := re.match(r"(\d+\.\d+\.\d+\.\d+)\:(\d+)", address)) is not None:
            address_ip, address_port = r.groups()
            address_port = int(address_port)

        elif (r := re.match(r"(\d+\.\d+\.\d+\.\d+)", address)) is not None:
            address_ip = r.group(1)
            address_port = None

        else:
            address_ip, address_port = None, None

        date_str = f"{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}"
        result.append({
            "date_str": date_str,
            "address": address,
            "address_ip": address_ip,
            "address_port": address_port,
            "method": method,
            "path": path,
            "status": status,
            "message": message,
            "year": year,
            "month": month,
            "day": day,
            "hour": hour,
            "minute": minute,
            "second": second,
        })

    return result

create = """
CREATE TABLE IF NOT EXISTS logs (
    id INTEGER PRIMARY KEY,
    date_str TEXT NOT NULL,
    address TEXT NOT NULL,  -- it can be a remote addr (for http requests) or an identifier (for internal functions)
    address_ip TEXT,
    address_port INTEGER,
    method TEXT,  -- GET | POST | PUT | DELETE
    path TEXT,
    status INTEGER,
    message TEXT NOT NULL,
    year INTEGER NOT NULL,
    month INTEGER NOT NULL,
    day INTEGER NOT NULL,
    hour INTEGER NOT NULL,
    minute INTEGER NOT NULL,
    second INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS logs_date_str ON logs (date_str);
CREATE INDEX IF NOT EXISTS logs_address ON logs (address);
CREATE INDEX IF NOT EXISTS logs_address_ip ON logs (address_ip);
CREATE INDEX IF NOT EXISTS logs_method ON logs (method);
CREATE INDEX IF NOT EXISTS logs_path ON logs (path);
CREATE INDEX IF NOT EXISTS logs_status ON logs (status);
"""

def dump_sqlite(
    parsed_log: list[dict],
    db_path: str = "logs.db",
):
    conn = sqlite3.connect(db_path)
    conn.executescript(create)

    for log in parsed_log:
        conn.execute("""
INSERT INTO logs (date_str, address, address_ip, address_port, method, path, status, message, year, month, day, hour, minute, second)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
            """, (
                log["date_str"],
                log["address"],
                log["address_ip"],
                log["address_port"],
                log["method"],
                log["path"],
                log["status"],
                log["message"],
                log["year"],
                log["month"],
                log["day"],
                log["hour"],
                log["minute"],
                log["second"],
            ),
        )

    conn.commit()

if __name__ == "__main__":
    path = "ragit-server-logs" if len(sys.argv) < 2 else sys.argv[1]
    db_path = "logs.db" if len(sys.argv) < 3 else sys.argv[2]

    with open(path, "r") as f:
        log = f.read()

    logs = parse_log(log)
    dump_sqlite(logs, db_path)
