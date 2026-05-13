-- Add network fields to locations
ALTER TABLE locations ADD COLUMN network_range TEXT;
ALTER TABLE locations ADD COLUMN vlan_ids TEXT;
ALTER TABLE locations ADD COLUMN dns_servers TEXT;
ALTER TABLE locations ADD COLUMN house_number TEXT;

-- Asset device types (manageable)
CREATE TABLE IF NOT EXISTS asset_device_types (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 100,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed default types
INSERT OR IGNORE INTO asset_device_types (code, label, sort_order) VALUES
    ('GW',   'Gateway',              10),
    ('FW',   'Firewall',             20),
    ('RTR',  'Router',               30),
    ('SW',   'Switch',               40),
    ('AP',   'Access Point',         50),
    ('WLC',  'WLAN Controller',      60),
    ('SRV',  'Server',               70),
    ('NAS',  'NAS',                  80),
    ('SAN',  'Storage',              90),
    ('UPS',  'USV',                 100),
    ('PDU',  'Stromverteiler',      110),
    ('PRN',  'Drucker',             120),
    ('CAM',  'Kamera',              130),
    ('IOT',  'IoT-Gerät',           140),
    ('MGMT', 'Management Appliance',150),
    ('OTHER','Sonstiges',           999);
