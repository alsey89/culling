# Cullrs

A privacy-first desktop application that helps photographers quickly cull large photo sets and remove duplicates using on-device AI processing.

**🔒 Privacy-First** • **⚡ Fast Native Performance** • **🆓 Open-Core Model**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Status](https://img.shields.io/badge/Status-Alpha-orange.svg)]()

---

## What is Cullrs?

Cullrs eliminates the tedious work of photo culling by automatically detecting duplicates and grouping similar images, while keeping you in control with safe, reversible operations. Built for photographers who need to process thousands of images efficiently without compromising on safety or privacy.

### Key Benefits

- **70-90% reduction in culling time** for event photographers
- **Zero cloud dependencies** - all processing happens on your device
- **Non-destructive workflow** with comprehensive undo capabilities
- **Professional integrations** with Lightroom, digiKam, and Photo Mechanic

---

## 🚀 Quick Start

```bash
# Clone and run
git clone https://github.com/alsey89/cullrs.git
cd cullrs
npm install
npm run tauri dev
```

> **Alpha Status:** Cullrs is in active development. Core features are functional but expect changes and occasional instability.

---

## ✨ What Makes Cullrs Different

**🆓 Powerful Free Core**

- Unlimited manual culling and organization
- Exact duplicate detection using SHA-256 hashing
- Visual similarity grouping with adjustable thresholds
- Safe file operations with session-based undo

**💎 Pro Features** _(Coming Soon)_

- Smart auto-selection rules based on quality metrics
- AI-powered image quality analysis (sharpness, exposure, noise)
- Face and eye detection for portrait optimization
- Professional workflow integrations and team collaboration

---

## 🏗️ Built With

- **Backend:** Rust (Tauri) for performance-critical operations
- **Frontend:** Vue 3 + Nuxt + TypeScript for modern UI
- **Storage:** SQLite for local project data
- **Platforms:** macOS, Windows, Linux

---

## 📚 Documentation

| Document                                          | Description                                      |
| ------------------------------------------------- | ------------------------------------------------ |
| **[📋 Product Requirements](docs/PRD.md)**        | Complete feature specifications and user stories |
| **[🏗️ Architecture Guide](docs/ARCHITECTURE.md)** | Technical design and implementation details      |
| **[🚧 Development Setup](docs/DEVELOPMENT.md)**   | Local environment and contribution guidelines    |
| **[🧪 Testing Guide](docs/TESTING.md)**           | Testing strategy and quality assurance           |

---

## 🛣️ Roadmap

- **M1 - MVP** (8 weeks) - Core culling features and exact duplicate detection
- **M2 - Pro Value** (6-8 weeks) - Auto-selection rules and quality metrics
- **M3 - Advanced** (10-12 weeks) - AI features and team collaboration

---

## 🤝 Contributing

We welcome contributions! Check out our [Development Guide](docs/DEVELOPMENT.md) to get started.

---

## 📄 License

Open-Core model: **Core features** under MIT License • **Pro features** require commercial license
