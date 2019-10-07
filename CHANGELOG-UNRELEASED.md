# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

{{ version-heading }}

### Added

* Re-write to use GhostActor pattern for inter-component async handling
* Adds mDNS discovery for websocket transport
* Adds a legacy wrapper so old Li3hClient/ServerProtocol can still be used while integration with core
* Adds Lib3hUri to handle our special-case URI formats
* Adds placeholders for tracing.

### Changed

* Updates Lib3hProtocol to use the four part GhostActor pattern

### Deprecated

* Lib3hClientProtocol & Lib3hServerProtocol will be going away

### Removed

### Fixed

### Security
