# ADDED Requirements

## Requirement: README covers project purpose and context

The README SHALL include a clear project title, a one-paragraph description of what the tool does, the problem it solves, and the environment it targets (Windows, CDK Drive ecosystem).

### Scenario: Reader understands purpose without reading source code

- **WHEN** a developer opens README.md for the first time
- **THEN** they SHALL understand within the first screen of text what the tool does, why it exists, and what problem it solves

## Requirement: README documents prerequisites

The README SHALL list all prerequisites: operating system (Windows), Rust toolchain version, and any required environment variables.

### Scenario: New developer can determine if they can build the project

- **WHEN** a developer reads the Prerequisites section
- **THEN** they SHALL know the minimum Rust edition, the target OS, and what must be installed before building

## Requirement: README documents environment variables

The README SHALL include a table or list of all environment variables recognized by the application, with name, description, whether required or optional, and the default value when optional.

### Scenario: Operator configures the tool without reading source code

- **WHEN** an operator reads the Configuration section
- **THEN** they SHALL know all env vars, which are mandatory (CDK_DRIVE_OSD_URL), which are optional (ADAPTIVA_VERSION_URL, LOG_DIR), and their default values

## Requirement: README documents CLI usage

The README SHALL show the command-line invocation syntax, describe each flag (`/query`, `--query`, `/update`, `--update`), and state the default mode when no flag is provided.

### Scenario: Operator knows how to run the tool in query mode

- **WHEN** an operator reads the Usage section
- **THEN** they SHALL be able to construct a valid command for both query and update mode

## Requirement: README describes output format

The README SHALL describe the ASCII-table log output format: what tables are produced, what columns they contain, and where log files are written.

### Scenario: Operator knows where logs are stored

- **WHEN** an operator reads the Output section
- **THEN** they SHALL know the default log directory (`cdk-updater-logs/`), the timestamp filename pattern, and that output is also written to stdout

## Requirement: README includes architecture overview

The README SHALL include a short architecture section that names the four modules (`main`, `installed`, `cdk_info`, `app_logging`) and describes the high-level data flow.

### Scenario: Developer understands module responsibilities at a glance

- **WHEN** a developer reads the Architecture section
- **THEN** they SHALL understand which module is responsible for registry queries, HTML scraping, CDK state gathering, and logging
