<p align="center">
  <picture>
    <source srcset="./src/assets/logo-white.svg" media="(prefers-color-scheme: dark)" />
    <source srcset="./src/assets/logo.svg" media="(prefers-color-scheme: light)" />
    <img src="./src/assets/logo.svg" alt="Quant Logo Banner" height="64"/>
  </picture>
</p>

[![Cyberpath](https://img.shields.io/badge/Cyberpath-project-blue)](https://cyberpath-hq.com)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](LICENSE.md)
[![Made with Astro](https://img.shields.io/badge/Made%20with-Astro-FF5D01?logo=astro&logoColor=white)](https://astro.build)

## What is Quant?

**Quant** is an intuitive, modern CVSS score calculator that makes vulnerability severity assessment fast, accurate, and
accessible. Unlike traditional calculators, Quant provides an interactive experience with color-coded severity
indicators, real-time scoring, and one-click downloads, making it your new go-to tool.

### Why Quant?

- **Intuitive Interface** — Beautiful, user-friendly design that makes CVSS scoring simple
- **Pure JavaScript Logic** — Fast, accurate, offline-capable scoring for all CVSS versions
- **Powerful Visualizations** — Instantly understand impact with color-coded severities and detailed analytics

## Table of Contents

- [Features](#features)
- [Roadmap](#roadmap)
- [Installation](#installation)
- [Usage](#usage)
- [Commands](#commands)
- [Contributing](#contributing)
- [Community](#community)
- [License](#license)

## Features

Quant delivers industry-leading features designed to streamline vulnerability assessment workflows:

### Multi-Version CVSS Support

- **CVSS 4.0** — Latest standard with enhanced scoring methodology
- **CVSS 3.1** — Industry-standard version with broad adoption
- **CVSS 3.0** — Legacy compatibility for established assessments
- **CVSS 2.0** — Historical scoring for legacy vulnerability tracking

### Intelligent Severity Visualization

- **Color-Coded Risk Levels** — Instantly identify severity at a glance
- **Dynamic Severity Mapping** — Automatic color assignment based on CVSS version and score range
- **Visual Impact Charts** — Real-time visualization of metric impact on final score
- **Severity Breakdown Panels** — Detailed breakdown showing how each metric contributes to the final score

### Advanced Metric Control

- **Interactive Metric Selection** — Intuitive controls for all CVSS metrics and vectors
- **Metric Descriptions** — In-context help explaining each metric and its implications
- **Temporal & Environmental Metrics** — Full support for optional metrics across all CVSS versions
- **Instant Score Updates** — Real-time recalculation as you adjust metrics

### One-Click Export & Sharing

- **Copy Score Vector** — Copy CVSS vector string for documentation and tracking
- **Share Links** — Generate shareable URLs with pre-configured scores and metrics
- **Embeddable Code** — Generate HTML snippets to embed score cards on websites
- **Save to Score Manager** — Store assessments for later reference and advanced management

### Score Analytics & Insights

- **Severity Distribution** — Visual breakdown of how scores distribute across severity levels
- **Metric Impact Analysis** — See which metrics have the greatest influence on your score
- **Comparative Scoring** — Compare scores across different CVSS versions side-by-side
- **Historical Tracking** — Track and reference previously calculated scores

### User Experience Enhancements

- **Dark & Light Modes** — Seamless theme switching for comfortable use in any environment
- **Responsive Design** — Perfect experience on desktop, tablet, and mobile devices
- **Keyboard Navigation** — Full keyboard support for power users and accessibility
- **Offline Capability** — Works completely offline—no internet required for scoring

### Privacy & Security

- **Client-Side Processing** — All calculations happen in your browser, no data sent to servers
- **No Account Required** — Use immediately without registration or login
- **Zero Data Collection** — Your assessments remain completely private
- **Open Source** — Full code transparency for security audits

### Developer Experience

- **Clean API Surface** — Pure JavaScript scoring functions usable in your own applications
- **TypeScript Support** — Fully typed for excellent IDE integration and type safety
- **Framework Agnostic** — Embed scoring logic into React, Vue, Angular, or vanilla JS projects
- **Comprehensive Documentation** — Clear examples and API reference for developers

## Roadmap

We're continuously improving Quant to make vulnerability assessment even more powerful and user-friendly. Here's what's
coming next:

### Planned Features

- **Interactive Calculator Tours** — Implement guided tours using onboarding-tour library to help new users understand
  the interface and features
- **Advanced Settings Page** — Create a centralized settings page with comprehensive configuration options and data
  export functionality

### Future Enhancements

- **Team Collaboration** — Multi-user support for shared assessments and collaborative scoring workflows
- **API Integration** — RESTful API for automated CVSS scoring in CI/CD pipelines and security tools
- **Vulnerability Database Integration** — Direct integration with CVE databases for automatic score suggestions

### Contributing to the Roadmap

We welcome feedback on our roadmap! If you have ideas for new features or want to contribute to any of these items,
please:

- Open an issue on GitHub to discuss your ideas
- Check our [contribution guidelines](CONTRIBUTING.md) for development setup
- Join our [Discord community](https://discord.gg/WmPc56hYut) to connect with other contributors

## Perfect For

- **Security Teams** — Daily vulnerability triage and risk quantification
- **Penetration Testers** — Quick, reliable CVSS scoring during assessments
- **Security Analysts** — Standardized severity assessment across your organization
- **Developers** — Secure development practices with clear vulnerability context
- **Organizations** — Enterprise-ready vulnerability management and reporting

## Getting Started

### Installation

Although Quant is open-source and can be run locally, we recommend using the
[official website](https://quant.cyberpath-hq.com) for the best experience.

The website is updated automatically with every new release, and you can access the latest features without needing to
install anything.

If you'd like to run it locally:

1. **Clone the repository:**

   ```bash
   git clone https://github.com/cyberpath-HQ/Quant.git
   cd Quant
   ```

2. **Install dependencies** (requires [pnpm](https://pnpm.io/)):
   ```bash
   pnpm install
   ```

### Usage

Start the development server:

```bash
pnpm dev
```

Access the application at `http://localhost:4321`

### Commands

All commands are run from the root of the project:

| Command          | Action                                     |
| :--------------- | :----------------------------------------- |
| `pnpm install`   | Install dependencies                       |
| `pnpm dev`       | Start local dev server at `localhost:4321` |
| `pnpm build`     | Build production site to `./dist/`         |
| `pnpm preview`   | Preview build locally before deploying     |
| `pnpm astro ...` | Run Astro CLI commands                     |
| `pnpm lint`      | Run ESLint and code quality checks         |

## Contributing

We welcome contributions from the cybersecurity community! Whether you're:

- Reporting bugs or suggesting improvements
- Enhancing the UI/UX
- Improving documentation
- Adding features or fixing issues
- Translating for international users

Please check our [contribution guidelines](CONTRIBUTING.md) for detailed information on how to contribute.

### Ways to Contribute

- Report issues and suggest enhancements
- Improve documentation and examples
- Help with translations
- Contribute code improvements and bug fixes
- Share feedback and real-world use cases

## Community

Join the Cyberpath community and stay connected:

- **Website:** [cyberpath-hq.com](https://cyberpath-hq.com)
- **GitHub:** [github.com/cyberpath-HQ](https://github.com/cyberpath-HQ)
- **Discord:** [Join our Discord server](https://discord.gg/WmPc56hYut)
- **Twitter:** [@cyberpath_hq](https://x.com/cyberpath_hq)
- **Email:** [support@cyberpath-hq.com](mailto:support@cyberpath-hq.com)

## Learn More

- [Astro Documentation](https://docs.astro.build)
- [CVSS v4.0 Specification](https://www.first.org/cvss/v4-0/specification-document)
- [CVSS v3.1 Specification](https://www.first.org/cvss/v3-1/specification-document)
- [CVSS v3.0 Specification](https://www.first.org/cvss/v3-0/specification-document)
- [CVSS v2.0 Specification](https://www.first.org/cvss/v2/guide)
- [Cyberpath Resources](https://cyberpath-hq.com)

## Acknowledgments

Built with ❤️ by the [Cyberpath-HQ](https://github.com/cyberpath-HQ) team to make vulnerability assessment accessible to
everyone.
