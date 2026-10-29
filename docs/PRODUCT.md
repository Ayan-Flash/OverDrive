# OverDrive — Product Documentation

> **Version**: 0.2.0
> **Last Updated**: 2026-07-17
> **Status**: Alpha — Phase 1 Core Platform Complete

---

## Table of Contents

1. [Product Vision](#product-vision)
2. [Target Audience](#target-audience)
3. [User Personas](#user-personas)
4. [Feature Overview](#feature-overview)
5. [User Stories](#user-stories)
6. [Competitive Landscape](#competitive-landscape)
7. [Monetization Strategy](#monetization-strategy)
8. [Roadmap](#roadmap)
9. [Success Metrics](#success-metrics)
10. [Risk Assessment](#risk-assessment)

---

## Product Vision

OverDrive is a professional-grade desktop application for real-time GPU monitoring, overclocking, and performance optimization on Windows. It aims to be the open, extensible alternative to MSI Afterburner — combining enterprise-grade architecture with a modern gaming aesthetic.

**Vision Statement:** *The definitive cross-vendor GPU command center — open, extensible, and built for performance enthusiasts, overclockers, and system integrators.*

### Why This Product Exists

The GPU tools landscape is dominated by vendor-locked utilities (NVIDIA Control Panel, AMD Adrenalin) and aging third-party tools (MSI Afterburner, last major update 2019). Users lack:

- A modern, unified interface that works across GPU vendors
- Extensibility through plugins and community profiles
- Enterprise-grade features (remote monitoring, automation, group policies)
- A well-maintained, open-source alternative with active development

OverDrive fills this gap with a clean-slate rewrite using modern technology (Tauri v2, React, Rust).

---

## Target Audience

### Primary Segments

| Segment | Size Estimate | Key Need | Willingness to Pay |
|---------|--------------|----------|-------------------|
| PC Gamers | 200M+ globally | Real-time monitoring, basic tuning | Low (free tier) |
| Overclocking Enthusiasts | 5–10M | Fine-grained control, benchmark validation | Medium |
| System Integrators / PC Builders | 500K–1M | Batch configuration, remote monitoring | High |
| Cryptocurrency Miners | 1–3M (declining) | Multi-GPU management, power optimization | Medium-High |
| Enterprise IT / Data Centers | 100K+ orgs | Group policies, compliance, remote management | High |
| eSports Teams / Game Devs | 50K+ | Performance profiling, overlay customization | Medium |

### Secondary Segments

- Hardware reviewers and benchmarkers
- DIY water-cooling enthusiasts
- Laptop power tuners (undervolting)
- AI/ML workstation managers

---

## User Personas

### Alex — The Overclocker

> *"I need every last MHz out of my RTX 5090. Give me raw data, fine controls, and don't get in my way."*

- **Age**: 24
- **Occupation**: Software engineer
- **Rig**: Custom water-cooled, single GPU, dual monitor
- **Behavior**: Spends hours tuning, benchmarks after every change, shares profiles on forums
- **Needs**: Real-time telemetry, per-game profiles, voltage control, benchmark integration
- **Pain points**: MSI Afterburner crashes during long tuning sessions, UI is hard to read at a glance

### Jordan — The System Integrator

> *"I build 50 gaming PCs a month. I need to flash the same fan curve and OC profile to every unit."*

- **Age**: 35
- **Occupation**: PC shop owner
- **Setup**: Multi-workstation bench with 4+ test GPUs
- **Behavior**: Batch-applies profiles, needs remote monitoring of test rigs
- **Needs**: Profile import/export, remote API, silent install, group policies
- **Pain points**: No existing tool supports batch configuration

### Sam — The eSports Manager

> *"I need to make sure all 20 team machines are running at peak performance during tournaments."*

- **Age**: 29
- **Occupation**: eSports team operations
- **Setup**: 20 identical tournament rigs
- **Behavior**: Monitors all machines from a single pane, needs instant alerting
- **Needs**: Centralized remote monitoring, alert thresholds, automated reporting
- **Pain points**: No unified dashboard, relies on manual checks

### Morgan — The Gamer

> *"I just want to see my GPU temp in-game and maybe tweak the fan curve. Free, please."*

- **Age**: 19
- **Occupation**: College student
- **Rig**: Single mid-range GPU, air cooled
- **Behavior**: Installs, checks temps once, uses basic fan curve, forgets about it
- **Needs**: Simple UI, in-game overlay, one-click auto-tune
- **Pain points**: Overwhelmed by complexity of existing tools

---

## Feature Overview

### By Release Phase

| Feature | Phase | Status | Priority |
|---------|-------|--------|----------|
| GPU detection & enumeration | 1 | Done | P0 |
| Real-time monitoring (temp, clock, fan, power, voltage, utilization) | 1 | Done | P0 |
| Historical data with ring buffer | 1 | Done | P0 |
| Fan speed control | 1 | Done | P0 |
| Core/memory clock offset adjustment | 1 | Done | P0 |
| Power limit control | 1 | Done | P0 |
| Profile save/load/apply | 1 | Done | P0 |
| In-game overlay | 1 | Done | P1 |
| Alert system with thresholds | 1 | Done | P1 |
| CSV data export | 1 | Done | P1 |
| SQLite persistence | 1 | Done | P0 |
| Remote monitoring HTTP API | 1 | Done | P1 |
| Multi-GPU support | 1 | Done | P0 |
| Plugin system | 1 | Done | P2 |
| Discord integration | 1 | Done | P2 |
| Automated testing suite | 1 | Done | P2 |
| Armoury Crate-style UI rebuild | 2 | In Progress | P0 |
| Automation engine (rules/conditions) | 2 | Planned | P1 |
| AI anomaly detection | 2 | Planned | P2 |
| Fan curve tuning (AI-assisted) | 2 | Planned | P2 |
| Performance prediction | 2 | Planned | P2 |
| Profile marketplace | 2 | Planned | P2 |
| Reporting (HTML reports) | 2 | Planned | P2 |
| OBS integration | 2 | Planned | P2 |
| Update system (auto-updater) | 2 | Planned | P2 |
| Device sync | 3 | Planned | P2 |
| Enterprise group policies | 3 | Planned | P2 |
| Mobile companion app | 4 | Planned | P3 |
| Water flow / pump control | 4 | Planned | P3 |
| RGB sync integration | 4 | Planned | P3 |

### Feature Tiers

```
Free Tier
├── GPU monitoring (up to 2 GPUs)
├── Basic fan control
├── In-game overlay (limited metrics)
├── 5 saved profiles
└── Community profile downloads

Pro Tier ($9.99 lifetime / $2.99/month)
├── Unlimited GPU monitoring
├── Full control suite (clock, voltage, power)
├── Unlimited profiles
├── Automation engine
├── Remote monitoring (up to 5 clients)
├── AI-assisted tuning
├── Advanced overlay (all metrics)
├── Report generation
└── Priority updates

Enterprise Tier ($49.99/node/year)
├── All Pro features
├── Unlimited remote clients
├── Group policy management
├── Centralized deployment
├── Compliance reporting
├── Dedicated support
└── Custom integrations
```

---

## User Stories

### Monitoring

```
As a gamer
I want to see my GPU temperature in real-time
So that I can ensure my cooling is adequate during gaming sessions

As an overclocker
I want to view historical temperature and clock data on a graph
So that I can identify thermal throttling patterns

As a system integrator
I want to monitor multiple GPUs simultaneously
So that I can compare their performance characteristics
```

### Control

```
As an enthusiast
I want to adjust my fan curve point-by-point
So that I can balance noise and thermals

As a miner
I want to apply a power limit to all GPUs at once
So that I can optimize efficiency across my rig

As a laptop user
I want to undervolt my GPU
So that I can reduce temperatures without losing performance
```

### Automation

```
As a power user
I want to automatically increase fan speed when temperature exceeds 75°C
So that I don't have to manually adjust during gaming

As an IT manager
I want to enforce a maximum temperature alert on all company workstations
So that I can prevent thermal damage to hardware
```

### Overlay

```
As a streamer
I want to display GPU metrics on my stream overlay
So that viewers can see performance data

As a competitive gamer
I want a minimal overlay that shows only FPS and GPU temp
So that I can monitor without distraction
```

---

## Competitive Landscape

### Direct Competitors

| Product | Strengths | Weaknesses | Our Advantage |
|---------|-----------|------------|---------------|
| **MSI Afterburner** | Ubiquitous, RTSS overlay, 15+ years of development | Abandoned (last update 2019), crash-prone, outdated UI, NVIDIA-only for control | Active development, modern architecture, cross-vendor |
| **EVGA Precision X1** | Clean UI, good voltage control | EVGA hardware only, no overlay, limited monitoring | Universal, richer feature set |
| **ASUS GPU Tweak III** | Good per-game profiles | ASUS GPU only, heavy bloatware | Lightweight, vendor-agnostic |
| **AMD Adrenalin** | Deep AMD integration, built-in recording | AMD-only, complex UI, gaming-focused | Cross-vendor, professional tier |

### Indirect Competitors

| Product | Category | Threat Level |
|---------|----------|-------------|
| **HWiNFO** | System monitoring | Low — no control features |
| **AIDA64** | System information + monitoring | Low — paid, no GPU control |
| **Open Hardware Monitor** | Open-source monitoring | Low — minimal features, no control |
| **NVIDIA Control Panel** | Vendor control panel | Low — NVIDIA-only, basic |
| **NZXT CAM** | System monitoring | Medium — good UX but limited |

### Market Positioning

```
                    Feature Richness
                    ▲
                    │         MSI Afterburner
                    │         OverDrive ★
                    │
                    │         ASUS GPU Tweak
                    │         EVGA Precision
                    │
                    │         HWiNFO
                    │         NZXT CAM
                    │
                    │         Open Hardware Monitor
                    └──────────────────────────► Modernity
```

OverDrive aims for the top-right quadrant: maximum feature richness with a modern architecture and active development cadence.

---

## Monetization Strategy

### Revenue Model

| Tier | Price | Revenue Share Estimate |
|------|-------|----------------------|
| Free | $0 | 80% of users |
| Pro | $9.99 lifetime / $2.99 monthly | 15% of users |
| Enterprise | $49.99/node/year | 5% of users (but 60% of revenue) |

### Projected Revenue (Year 1)

Assuming 100K downloads, 5% Pro conversion, 100 enterprise nodes:

- Pro: 5,000 users × $9.99 = ~$50K
- Enterprise: 100 nodes × $49.99 = ~$5K
- **Total Year 1: ~$55K**

### Marketplace Revenue

- 15% commission on paid community profiles
- Featured profile placement fees

---

## Roadmap

### Phase 1 — Core Platform (Current)

*Status: Substantially Complete*

- GPU detection and real-time monitoring
- Fan speed, clock offset, power limit controls
- Profile save/load/apply
- In-game overlay (basic)
- Alert system
- SQLite database persistence
- Remote monitoring API (basic)
- Multi-GPU support
- Plugin system architecture

### Phase 2 — Intelligence & Polish (In Progress)

- Armoury Crate-style UI rebuild
- Automation engine (event-driven and scheduled rules)
- AI-assisted tuning (fan curves, clock offsets, power limits)
- Anomaly detection and predictive monitoring
- Profile marketplace
- HTML/CSV reporting
- Auto-updater
- OBS and Discord integrations
- Comprehensive testing

### Phase 3 — Enterprise & Scale (Planned)

- Device sync (client-server protocol)
- Enterprise group policies and compliance
- Role-based access control
- Centralized deployment tooling
- Audit logging

### Phase 4 — Ecosystem (Future)

- Mobile companion app (iOS/Android)
- Water cooling integration (pump, flow, reservoir)
- RGB synchronization (Corsair iCUE, Razer Chroma)
- Benchmark automation suite
- Community SDK for plugin developers

---

## Success Metrics

### Quantitative

| Metric | Target (Year 1) | Measurement |
|--------|----------------|-------------|
| Downloads | 100,000 | GitHub releases + distribution |
| Active Users (MAU) | 25,000 | Telemetry (opt-in) |
| Pro Conversion Rate | 5% | Stripe purchase data |
| Enterprise Nodes | 100 | License server |
| Community Profiles | 500 | Marketplace |
| GitHub Stars | 2,000 | GitHub |
| Plugin Ecosystem | 20 plugins | Marketplace |
| Bug Report → Fix | < 14 days | GitHub issues |
| Perceived Performance | < 100ms metric latency | Internal benchmark |

### Qualitative

- User satisfaction (in-app NPS survey, target: 40+)
- Community engagement (Discord active members)
- Review sentiment (Reddit, Steam forums, YouTube)

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| NVIDIA API changes (NVML) break hardware access | Low | Critical | Abstracted provider trait, simulated provider for testing |
| Low adoption vs MSI Afterburner inertia | Medium | High | Focus on unique features (plugins, remote, AI) after parity |
| GPU market decline (mining crash, crypto winter) | Medium | Medium | Target gamers and enterprise, not just miners |
| Tauri security vulnerabilities | Low | High | Regular dependency updates, security audit before v1.0 |
| Developer burnout (solo/small team) | Medium | High | Open-source contributions, CI automation, documentation |
| Competing OSS project emerges | Low | Medium | Differentiate with architecture quality and feature velocity |

---

## Current Status

| Area | Status | Notes |
|------|--------|-------|
| Backend (Rust) | 90% complete | All modules implemented, minor integration gaps |
| Frontend | 60% complete | UI rebuild in progress, core pages functional |
| Database | 100% complete | Schema finalized, stores implemented |
| Testing | 70% coverage | Frontend at 75%, backend at 65%, improving |
| Documentation | 70% complete | ARCHITECTURE.md, DESIGN.md, PRODUCT.md, ROADMAP.md done |
| CI/CD | 100% complete | GitHub Actions for lint, typecheck, test, build, release |
| Installer | Done | NSIS installer builds successfully |
| Auto-updater | In progress | Tauri updater configured, testing pending |

---

## Technical Differentiators

| Feature | OverDrive | MSI Afterburner |
|---------|---------------|-----------------|
| Architecture | Tauri v2 + React + Rust | C++ Win32 |
| Plugin System | Yes (trait-based, hot-loadable) | Limited (external) |
| Remote Monitoring | Built-in HTTP API | No |
| Automation Engine | Event-driven + scheduled rules | No |
| AI Features | Anomaly detection, prediction, tuning | No |
| Cross-Vendor | NVIDIA + AMD + Intel (planned) | NVIDIA only (control) |
| Open Source | Yes (MIT) | No |
| Modern UI | Custom dark gaming theme | 2013-era WinForms |
| Data Persistence | SQLite (WAL mode) | INI files |
| CI/CD | Full automated pipeline | Manual builds |

---

*Last updated: 2026-07-17*
