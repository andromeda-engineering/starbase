# Security Policy

## Supported Versions

NeonOS is currently pre-MVP. Security reports are still welcome.

## Reporting a Vulnerability

Please do not open public issues for security vulnerabilities.

Use GitHub private vulnerability reporting if available, or contact the maintainer directly.

## Security Principles

NeonOS should:

- avoid sending repo contents to external services by default
- avoid storing secrets in memory
- respect `.gitignore`
- avoid destructive file writes
- require explicit user intent before risky operations
