# adf-guardian
Bring some governance to your Azure Data Factory assets

**adf-guardian** is a **governance and linting CLI** for Azure Data Factory (ADF), written in Rust. It allows data engineering teams to enforce development standards, security policies, and naming conventions **locally** (before pushing code) or within **CI/CD pipelines**.

## Key Features

- **Shift-Left Governance:** Catch errors or warnings before code is pushed to production.
- **Flexible Rules:** Define rules using standard YAML and robust **RFC 9535 JSONPath** selectors.
- **Dependency-Free:** Single binary distribution.
<!-- - **CI/CD Ready:** Returns standard exit codes (0 or 1) for easy integration with GitHub Actions, Azure DevOps, etc.-->

---

## 📦 Installation

### Option 1: Pre-built Binaries (Recommended)
Download the latest version for Windows, Linux, or macOS from the [Releases Page](https://github.com/matheussrod/adf-guardian/releases).

### Option 2: Build from Source
If you have the Rust toolchain installed:

```bash
git clone https://github.com/matheussrod/adf-guardian.git
cd adf-guardian
cargo install --path .
```

# Usage
Running adf-guardian is simple. By default, it looks for an guards.yaml config file in the current directory and scans the current folder.

```bash
# Basic run
adf-guardian

# Run on a specific project folder
adf-guardian --project-path ./my-adf-project

# Run with a specific configuration file
adf-guardian --config ./my-configs.yaml
```

Output is printed to the terminal, and a non-zero exit code is returned if any violations are found:
```bash
⛊ adf-guardian v0.1.0

› .\trigger\example.json
  • [id] description
    Actual value: "value"
Done: 1 scanned · 0 failed · 1 warning(s) · 0.0s
```

---

# Configuration (adf-guard.yaml)
The configuration file defines the rules for validating your ADF assets. It uses **JSONPath** (`RFC 9535`) to select specific JSON nodes and **Guards** (validation primitives) to assert their state.

> **What is JSONPath?** It's a query language for JSON, similar to XPath for XML. You use it to pinpoint a specific part of a JSON file. For example, `$.name` selects the `name` field at the root of the file. To learn more, see the official [RFC 9535 spec](https://www.rfc-editor.org/rfc/rfc9535.html).

## Rule Structure
Each rule is an object in the `rules` list with the following fields:

| Field         | Type                | Required | Description                                                                                                                              |
|---------------|---------------------|----------|------------------------------------------------------------------------------------------------------------------------------------------|
| `id`          | String              | Yes      | A unique identifier for the rule (e.g., `naming-convention-pipelines`).                                                                  |
| `asset`       | String or List      | Yes      | The ADF asset type(s) to which the rule applies. Valid values: `pipeline`, `dataset`, `linkedService`, `trigger`, `dataflow`, etc.         |
| `description` | String              | Yes      | A human-readable description of what the rule enforces. This is shown in the output when a validation fails.                             |
| `severity`    | String              | Yes      | The severity level if the rule fails. Valid values: `Error` (returns a non-zero exit code) or `Warning` (prints a message but passes).      |
| `when`        | Object              | No       | A conditional block. The `validate` block will only be executed if the condition defined in the `when` block is met.                     |
| `validate`    | Object or List      | Yes      | The core validation logic. It specifies the `target` node to check, the `guard` to use, and the `params` for that guard.                  |

A validation block (`when` or `validate`) has the following structure:
- `target`: The JSONPath string to select a node in the asset file.
- `guard`: The name of the built-in validation primitive to use.
- `params`: An object containing parameters for the specified `guard`.

---

# Guards (Validation Primitives)
Guards are the built-in functions you can use to validate the nodes selected by your JSONPath `target`.

| Guard         | Parameters                                                          | Description                                                                                                        |
|---------------|---------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------|
| `PatternMatch`  | `regex` (String)<br>`negative` (Bool, optional)                       | Validates if the target string matches the given [Rust-flavored regular expression](https://docs.rs/regex/latest/regex/#syntax). Set `negative: true` to assert it does *not* match.    |
| `AllowedValues` | `values` (List)<br>`mode` ("Allow"\|"Deny", optional)<br>`case_sensitive` (Bool, optional) | Checks if the target value is in a list. `mode: "Allow"` (default) acts as a whitelist. `mode: "Deny"` acts as a blacklist. |
| `Exists`        | `should_exist` (Bool, optional)                                       | Checks if a field is present (`should_exist: true`, default) or absent (`should_exist: false`). A field is considered non-existent if it is `null` or not defined. |
| `Range`         | `min` (Number, optional)<br>`max` (Number, optional)                   | Validates that a numeric value is within a specified inclusive range.                                              |
| `Count`         | `min` (Int, optional)<br>`max` (Int, optional)                         | Validates the number of items in an array.                                                                         |
| `StringLength`  | `min` (Int, optional)<br>`max` (Int, optional)                         | Validates the character length of a string.                                                                        |

---

# Examples

Below are practical examples of rules you can implement with `adf-guardian`.

---
### 1. Enforce Asset Naming Conventions
This rule ensures that all `pipeline` and `dataset` assets follow a specific naming convention (e.g., `pl_` prefix for pipelines, `ds_` for datasets).

**Guards Used:** `PatternMatch`

```yaml
rules:
  - id: "naming-convention-pipelines"
    asset: "pipeline"
    description: "Pipelines must start with the prefix 'pl_'"
    severity: "Warning"
    validate:
      target: "$.name"
      guard: "PatternMatch"
      params:
        regex: "^pl_"

  - id: "naming-convention-datasets"
    asset: "dataset"
    description: "Datasets must start with the prefix 'ds_'"
    severity: "Warning"
    validate:
      target: "$.name"
      guard: "PatternMatch"
      params:
        regex: "^ds_"
```

---
### 2. Prevent Hardcoded Credentials in Linked Services
This rule prevents committing linked services that contain hardcoded credentials, which is a major security risk. The `encryptedCredential` field should not exist.

**Guards Used:** `Exists`

```yaml
rules:
  - id: "security-no-hardcoded-credentials"
    asset: "linkedService"
    description: "Linked Services should not contain hardcoded credentials ('encryptedCredential'). Use Azure Key Vault instead."
    severity: "Error"
    validate:
      target: "$.properties.typeProperties.encryptedCredential"
      guard: "Exists"
      params: 
        should_exist: false
```

---
### 3. Enforce Standard Trigger Frequencies
This rule ensures that triggers only run at approved intervals (e.g., 'Hour' or 'Day'), preventing configurations that might be too frequent and costly. It uses a `when` clause to apply the rule only to `ScheduleTrigger` types.

**Guards Used:** `AllowedValues` (in `when` and `validate`)

```yaml
rules:
  - id: "compliance-trigger-frequency"
    asset: "trigger"
    description: "Scheduled triggers must have a frequency of 'Hour' or 'Day'."
    severity: "Warning"
    when:
      target: "$.properties.type"
      guard: "AllowedValues"
      params: 
        values: ["ScheduleTrigger"]
    validate:
      target: "$.properties.typeProperties.recurrence.frequency"
      guard: "AllowedValues"
      params:
        mode: "Allow"
        values: ["Hour", "Day"]
```

---
### 4. Enforce Snake Case for All Resource Names
This rule enforces a consistent `snake_case` naming style across multiple asset types for better readability and standardization.

**Guards Used:** `PatternMatch`

```yaml
rules:
  - id: "naming-convention-snake-case"
    description: "Resource names must follow snake_case pattern (e.g., 'my_resource_name')."
    severity: "Error"
    asset: 
      - "pipeline"
      - "trigger"
      - "linkedService"
      - "dataset"
      - "dataflow"
    validate:
      target: "$.name"
      guard: "PatternMatch"
      params:
        regex: "^[a-z0-9]+(_[a-z0-9]+)*$"
```

---
### 5. Limit Pipeline Activity Retries
This rule sets a sensible maximum for the retry interval in pipeline activities, preventing excessively long-running or frequent retries.

**Guards Used:** `Range`

```yaml
rules:
  - id: "policy-limit-retry-interval"
    description: "The activity retry interval (retryIntervalInSeconds) cannot be greater than 60 seconds."
    severity: "Error"
    asset: "pipeline"
    validate:
      target: "$..policy.retryIntervalInSeconds"
      guard: "Range"
      params:
        max: 60
```

---
### 6. Limit Pipeline Complexity
To improve readability and maintainability, this rule limits the number of activities allowed in a single pipeline.

**Guards Used:** `Count`

```yaml
rules:
  - id: "best-practice-pipeline-complexity"
    description: "Pipelines should not exceed 15 activities to ensure readability and ease of maintenance."
    severity: "Warning"
    asset: "pipeline"
    validate:
      target: "$.properties.activities"
      guard: "Count"
      params:
        max: 15
```

---
### 7. Control Query String Length
This rule validates the length of embedded SQL queries within pipeline activities, preventing overly complex or potentially runaway queries from being committed.

**Guards Used:** `StringLength`

```yaml
rules:
  - id: "best-practice-sql-query-length"
    description: "Embedded SQL reader queries should not exceed 2000 characters."
    severity: "Error"
    asset: "pipeline"
    validate:
      target: "$..sqlReaderQuery"
      guard: "StringLength"
      params:
        max: 2000
```
