-- GeekORM Database Migrations

-- id Table
CREATE TABLE IF NOT EXISTS id (build_timestamp INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, schema_version INTEGER NOT NULL);

-- vulnerability Table
CREATE TABLE IF NOT EXISTS vulnerability (pk INTEGER PRIMARY KEY, id TEXT NOT NULL UNIQUE, package_name TEXT NOT NULL, namespace TEXT NOT NULL, package_qualifiers TEXT, version_constraint TEXT NOT NULL, version_format TEXT NOT NULL, cpes TEXT, related_vulnerabilities TEXT, fixed_in_versions TEXT, fix_state TEXT NOT NULL, advisories TEXT);

-- vulnerability_metadata Table
CREATE TABLE IF NOT EXISTS vulnerability_metadata (id INTEGER PRIMARY KEY, namespace TEXT NOT NULL, data_source TEXT NOT NULL, record_source TEXT NOT NULL, severity TEXT NOT NULL, urls TEXT, description TEXT NOT NULL, cvss TEXT);

-- Sessions Table
CREATE TABLE IF NOT EXISTS Sessions (id INTEGER PRIMARY KEY AUTOINCREMENT, session_type BLOB NOT NULL, state BLOB NOT NULL, token TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL, last_accessed TEXT NOT NULL);

-- Users Table
CREATE TABLE IF NOT EXISTS Users (id INTEGER PRIMARY KEY AUTOINCREMENT, state BLOB NOT NULL, username TEXT NOT NULL UNIQUE, password TEXT NOT NULL, role BLOB NOT NULL, sessions INTEGER NOT NULL, created_at TEXT NOT NULL, last_login TEXT NOT NULL, FOREIGN KEY (sessions) REFERENCES Sessions(id));

-- Component Table
CREATE TABLE IF NOT EXISTS Component (id INTEGER PRIMARY KEY AUTOINCREMENT, component_type BLOB NOT NULL, manager BLOB NOT NULL, namespace TEXT, name TEXT NOT NULL);

-- ComponentVersion Table
CREATE TABLE IF NOT EXISTS ComponentVersion (id INTEGER PRIMARY KEY AUTOINCREMENT, component_id INTEGER NOT NULL, version TEXT NOT NULL, FOREIGN KEY (component_id) REFERENCES Component(id));

-- SnapshotMetadata Table
CREATE TABLE IF NOT EXISTS SnapshotMetadata (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, key BLOB NOT NULL, value BLOB NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, FOREIGN KEY (snapshot_id) REFERENCES Snapshot(id));

-- Snapshot Table
CREATE TABLE IF NOT EXISTS Snapshot (id INTEGER PRIMARY KEY AUTOINCREMENT, state BLOB NOT NULL, created_at TEXT NOT NULL);

-- Dependencies Table
CREATE TABLE IF NOT EXISTS Dependencies (id INTEGER PRIMARY KEY AUTOINCREMENT, snapshot_id INTEGER NOT NULL, component_id INTEGER NOT NULL, component_version_id INTEGER NOT NULL, FOREIGN KEY (snapshot_id) REFERENCES Snapshot(id), FOREIGN KEY (component_id) REFERENCES Component(id), FOREIGN KEY (component_version_id) REFERENCES ComponentVersion(id));

-- Projects Table
CREATE TABLE IF NOT EXISTS Projects (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, title TEXT, description TEXT, project_type BLOB NOT NULL, status BLOB NOT NULL, parent INTEGER NOT NULL, created_at TEXT NOT NULL);

-- ProjectSnapshots Table
CREATE TABLE IF NOT EXISTS ProjectSnapshots (id INTEGER PRIMARY KEY AUTOINCREMENT, project_id INTEGER NOT NULL, snapshot_id INTEGER NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY (project_id) REFERENCES Projects(id), FOREIGN KEY (snapshot_id) REFERENCES Snapshot(id));

-- Advisories Table
CREATE TABLE IF NOT EXISTS Advisories (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, source BLOB NOT NULL, severity BLOB NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);

-- AdvisoriesMetadata Table
CREATE TABLE IF NOT EXISTS AdvisoriesMetadata (id INTEGER PRIMARY KEY AUTOINCREMENT, key TEXT NOT NULL, value TEXT NOT NULL, advisory_id INTEGER NOT NULL, updated_at TEXT NOT NULL, FOREIGN KEY (advisory_id) REFERENCES Advisories(id));

-- Alerts Table
CREATE TABLE IF NOT EXISTS Alerts (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, state BLOB NOT NULL, snapshot_id INTEGER NOT NULL, dependency_id INTEGER NOT NULL, advisory_id INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, FOREIGN KEY (snapshot_id) REFERENCES Snapshot(id), FOREIGN KEY (dependency_id) REFERENCES Dependencies(id), FOREIGN KEY (advisory_id) REFERENCES Advisories(id));

-- ServerSettings Table
CREATE TABLE IF NOT EXISTS ServerSettings (id INTEGER PRIMARY KEY AUTOINCREMENT, name BLOB NOT NULL UNIQUE, setting_type BLOB NOT NULL, value TEXT NOT NULL, updated_at TEXT NOT NULL);

