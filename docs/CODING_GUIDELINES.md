# OverDrive - Coding Guidelines

> **Version**: 1.0.0  
> **Last Updated**: 2024  
> **Status**: Phase 1 Complete

---

## Table of Contents

1. [Overview](#overview)
2. [Rust Standards](#rust-standards)
3. [React Standards](#react-standards)
4. [TypeScript Standards](#typescript-standards)
5. [Naming Conventions](#naming-conventions)
6. [Folder Conventions](#folder-conventions)
7. [Error Handling](#error-handling)
8. [Logging Standards](#logging-standards)
9. [Comments Policy](#comments-policy)
10. [Testing Strategy](#testing-strategy)
11. [Git Workflow](#git-workflow)
12. [Pull Request Standards](#pull-request-standards)

---

## Overview

These coding guidelines ensure consistency, maintainability, and quality across the OverDrive codebase. All contributors must follow these guidelines.

---

## Rust Standards

### Code Style

- **Edition**: Rust 2021
- **Formatting**: `cargo fmt` (rustfmt)
- **Linting**: `cargo clippy`
- **Documentation**: `cargo doc`

### Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Crates | `snake_case` | `overdrive_monitoring` |
| Modules | `snake_case` | `gpu_driver` |
| Types | `PascalCase` | `GpuInfo`, `MonitoringData` |
| Functions | `snake_case` | `get_gpu_info`, `start_monitoring` |
| Methods | `snake_case` | `self.get_data()` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_GPU_COUNT` |
| Enums | `PascalCase` | `GpuVendor`, `FanCurveType` |
| Traits | `PascalCase` + `trait` suffix | `GpuDriver`, `MonitoringSource` |

### Code Organization

```rust
// 1. License and imports
// 2. Module declarations
// 3. Public types and functions
// 4. Private types and functions
// 5. Implementation blocks
```

### Error Handling

- Use `thiserror` for error types
- Provide meaningful error messages
- Use `Result<T, E>` for recoverable errors
- Use `panic!` only for unrecoverable errors

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("GPU not found: {0}")]
    GpuNotFound(String),
    
    #[error("Failed to read GPU data: {0}")]
    ReadError(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
```

### Documentation

- Document all public items
- Use `///` for item documentation
- Use `//!` for module documentation
- Include examples where helpful

```rust
/// Represents a GPU device with monitoring capabilities.
///
/// # Examples
///
/// ```
/// let gpu = Gpu::new(0, "NVIDIA GeForce RTX 3080");
/// assert_eq!(gpu.index(), 0);
/// ```
pub struct Gpu {
    index: u32,
    name: String,
}
```

---

## React Standards

### Code Style

- **ESLint**: Airbnb configuration with custom rules
- **Prettier**: Default settings with Tailwind plugin
- **TypeScript**: Strict mode enabled

### Component Structure

```
Component.tsx
├── Imports
├── Types/Interfaces
├── Component
├── Hooks
├── Handlers
├── Render
└── Export
```

### Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Components | `PascalCase` | `MonitoringDashboard`, `FanControlPanel` |
| Hooks | `use` + `PascalCase` | `useGpuData`, `useFanSpeed` |
| Constants | `PascalCase` | `DefaultRefreshRate`, `MaxFanSpeed` |
| Types | `PascalCase` | `GpuData`, `FanCurve` |
| Interfaces | `PascalCase` + `I` prefix | `IGpuInfo`, `IMonitoringConfig` |

### Component Guidelines

- **Single Responsibility**: Each component does one thing
- **Small Components**: Prefer many small components over few large ones
- **Props Interface**: Define props as interfaces
- **Default Props**: Use destructuring with defaults
- **Memoization**: Use `useMemo` and `useCallback` for performance

```tsx
interface FanControlPanelProps {
  gpuId: string;
  currentSpeed: number;
  minSpeed: number;
  maxSpeed: number;
  onChange: (speed: number) => void;
}

export const FanControlPanel: React.FC<FanControlPanelProps> = ({
  gpuId,
  currentSpeed,
  minSpeed = 0,
  maxSpeed = 100,
  onChange,
}) => {
  // Component implementation
};
```

### Hooks Guidelines

- **Custom Hooks**: Use `use` prefix, PascalCase
- **Single Responsibility**: Each hook does one thing
- **Dependencies**: Include all dependencies in `useEffect` array
- **Cleanup**: Return cleanup function when needed

```tsx
export const useGpuData = (gpuId: string): GpuData | null => {
  const [data, setData] = useState<GpuData | null>(null);
  
  useEffect(() => {
    const subscription = monitoringService.subscribe(gpuId, setData);
    
    return () => {
      subscription.unsubscribe();
    };
  }, [gpuId]);
  
  return data;
};
```

---

## TypeScript Standards

### Code Style

- **Strict Mode**: Enable all strict mode options
- **No Implicit Any**: Disable `noImplicitAny` with proper types
- **No Implicit Returns**: Enable `noImplicitReturns`
- **No Unused Locals**: Enable `noUnusedLocals`

### Type Definitions

- **Avoid `any`**: Use specific types
- **Interfaces**: Prefer interfaces over types for objects
- **Unions**: Use unions for limited options
- **Generics**: Use generics for reusable functions

```typescript
// Prefer interfaces
interface GpuInfo {
  id: string;
  name: string;
  vendor: 'nvidia' | 'amd' | 'intel';
  index: number;
}

// Use unions for limited options
type GpuVendor = 'nvidia' | 'amd' | 'intel';

// Use generics for reusable functions
function fetchData<T>(url: string): Promise<T> {
  return fetch(url).then(res => res.json());
}
```

### Async/Await

- Use `async/await` instead of `.then()`
- Handle errors with `try/catch`
- Use `Promise.all` for parallel operations

```typescript
// Good
try {
  const [gpuData, systemData] = await Promise.all([
    getGpuData(),
    getSystemData(),
  ]);
  return { gpuData, systemData };
} catch (error) {
  console.error('Failed to fetch data:', error);
  throw error;
}

// Bad
getGpuData()
  .then(gpuData => getSystemData())
  .then(systemData => ({ gpuData, systemData }))
  .catch(error => {
    console.error('Failed to fetch data:', error);
    throw error;
  });
```

---

## Naming Conventions

### General Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Variables | `camelCase` | `gpuCount`, `fanSpeed` |
| Functions | `camelCase` | `getGpuInfo`, `startMonitoring` |
| Classes | `PascalCase` | `GpuMonitor`, `FanController` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_GPU_COUNT`, `DEFAULT_REFRESH_RATE` |
| Enums | `PascalCase` | `GpuVendor`, `FanCurveType` |
| Interfaces | `PascalCase` | `IGpuInfo`, `IMonitoringConfig` |
| Types | `PascalCase` | `GpuData`, `FanCurve` |

### File Naming

| File Type | Convention | Example |
|-----------|-----------|---------|
| Components | `PascalCase.tsx` | `MonitoringDashboard.tsx` |
| Hooks | `usePascalCase.ts` | `useGpuData.ts` |
| Services | `camelCase.ts` | `monitoringService.ts` |
| Utilities | `camelCase.ts` | `formatNumber.ts` |
| Types | `camelCase.ts` | `gpuTypes.ts` |
| Constants | `camelCase.ts` | `gpuConstants.ts` |

### Folder Naming

| Folder Type | Convention | Example |
|-------------|-----------|---------|
| Components | `components/` | `components/base/` |
| Pages | `pages/` | `pages/monitoring/` |
| Services | `services/` | `services/api/` |
| Stores | `stores/` | `stores/gpu/` |
| Hooks | `hooks/` | `hooks/useGpuData/` |
| Utils | `utils/` | `utils/format/` |

---

## Folder Conventions

### Project Structure

```
overdrive/
├── apps/
│   ├── frontend/
│   │   ├── src/
│   │   │   ├── components/     # All components
│   │   │   │   ├── base/       # Atomic components
│   │   │   │   ├── composite/  # Component combinations
│   │   │   │   ├── feature/    # Feature components
│   │   │   │   ├── layout/     # Layout components
│   │   │   │   └── page/       # Page components
│   │   │   ├── hooks/          # Custom hooks
│   │   │   │   └── useX/       # Hook folders
│   │   │   ├── pages/          # Page components
│   │   │   ├── services/       # API services
│   │   │   │   └── api/        # API service folders
│   │   │   ├── stores/         # Zustand stores
│   │   │   │   └── gpuStore.ts
│   │   │   ├── themes/         # Theme configurations
│   │   │   ├── types/          # Type definitions
│   │   │   ├── utils/          # Utility functions
│   │   │   │   └── format/     # Utility folders
│   │   │   └── App.tsx
│   │   ├── public/
│   │   ├── tests/
│   │   └── package.json
│   └── backend/
│       ├── src/
│       │   ├── commands/       # Tauri commands
│       │   ├── hardware/       # Hardware access
│       │   │   ├── nvidia/     # NVIDIA modules
│       │   │   ├── amd/        # AMD modules
│       │   │   ├── intel/      # Intel modules
│       │   │   └── system/     # System modules
│       │   ├── monitoring/     # Monitoring engine
│       │   │   ├── engine/     # Core engine
│       │   │   ├── collector/  # Data collection
│       │   │   ├── aggregator/ # Data aggregation
│       │   │   └── exporter/   # Data export
│       │   ├── plugins/        # Plugin system
│       │   │   ├── manager/    # Plugin manager
│       │   │   ├── loader/     # Plugin loader
│       │   │   └── types/      # Plugin types
│       │   ├── stores/         # Backend stores
│       │   ├── utils/          # Backend utilities
│       │   ├── main.rs
│       │   └── lib.rs
│       ├── build.rs
│       ├── Cargo.toml
│       └── tauri.conf.json
├── packages/
│   ├── common/
│   │   ├── src/
│   │   │   ├── constants/      # Shared constants
│   │   │   ├── types/          # Shared types
│   │   │   └── utils/          # Shared utilities
│   │   ├── package.json
│   │   └── tsconfig.json
│   ├── monitoring/
│   │   ├── src/
│   │   │   ├── engine/         # Monitoring engine
│   │   │   ├── data/           # Data structures
│   │   │   └── index.ts
│   │   ├── package.json
│   │   └── Cargo.toml
│   └── plugins/
│       ├── src/
│       │   ├── types/          # Plugin types
│       │   ├── manager/        # Plugin manager
│       │   └── index.ts
│       ├── package.json
│       └── Cargo.toml
├── docs/
├── scripts/
├── tests/
│   ├── backend/
│   │   ├── integration/
│   │   └── unit/
│   ├── frontend/
│   │   ├── integration/
│   │   └── unit/
│   └── integration/
├── assets/
├── .github/
├── package.json
├── Cargo.toml
└── tsconfig.json
```

### File Organization

- **One responsibility per file**: Each file should have one clear purpose
- **Group related files**: Keep related files in the same folder
- **Flat structure**: Avoid deep nesting when possible
- **Clear names**: Use descriptive names for files and folders

---

## Error Handling

### Rust Error Handling

- Use `Result<T, E>` for recoverable errors
- Use `thiserror` for error types
- Provide meaningful error messages
- Use `?` operator for propagation

```rust
pub fn get_gpu_info(index: u32) -> Result<GpuInfo, GpuError> {
    let handle = unsafe { nvml::device_get_handle_by_index(index) };
    
    if handle.is_null() {
        return Err(GpuError::GpuNotFound(index.to_string()));
    }
    
    let name = unsafe { nvml::device_get_name(handle) };
    
    Ok(GpuInfo {
        index,
        name,
        vendor: GpuVendor::Nvidia,
    })
}
```

### TypeScript Error Handling

- Use `try/catch` for async/await
- Create custom error classes
- Provide error context
- Log errors with context

```typescript
class ApiError extends Error {
  constructor(
    message: string,
    public readonly statusCode?: number,
    public readonly details?: unknown
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export const fetchGpuData = async (gpuId: string): Promise<GpuData> => {
  try {
    const response = await invoke<GpuData>('gpu_get_data', { gpuId });
    return response;
  } catch (error) {
    throw new ApiError('Failed to fetch GPU data', undefined, error);
  }
};
```

### Error Patterns

| Pattern | Use Case | Example |
|---------|----------|---------|
| `Result<T, E>` | Recoverable errors | File I/O, API calls |
| `Option<T>` | Missing values | `get_gpu_by_id()` |
| `panic!` | Unrecoverable errors | Invariant violations |
| `unwrap()` | Development only | Tests, known-safe operations |

---

## Logging Standards

### Rust Logging

- Use `tracing` crate for logging
- Use appropriate log levels
- Include context in log messages
- Use structured logging

```rust
use tracing::{info, warn, error, debug};

pub fn start_monitoring(gpu_id: &str) {
    info!(gpu_id, "Starting monitoring");
    
    match monitoring_engine.start(gpu_id) {
        Ok(_) => info!(gpu_id, "Monitoring started"),
        Err(e) => error!(gpu_id, error = %e, "Failed to start monitoring"),
    }
}
```

### TypeScript Logging

- Use `logger` service
- Include context in log messages
- Use appropriate log levels
- Include stack traces for errors

```typescript
import { logger } from '@common/utils/logging';

export const monitoringService = {
  start: async (gpuId: string) => {
    logger.info('Starting monitoring', { gpuId });
    
    try {
      await invoke<void>('monitoring_start', { gpuId });
      logger.info('Monitoring started', { gpuId });
    } catch (error) {
      logger.error('Failed to start monitoring', { 
        gpuId, 
        error: error instanceof Error ? error.message : String(error) 
      });
      throw error;
    }
  },
};
```

### Log Levels

| Level | Use Case | Example |
|-------|----------|---------|
| `trace` | Detailed debugging | Function entry/exit |
| `debug` | Debug information | Variable states |
| `info` | General information | Application start |
| `warn` | Warnings | Deprecated API usage |
| `error` | Errors | Failed operations |
| `critical` | Critical failures | Hardware access failure |

---

## Comments Policy

### Rust Comments

- Use `///` for public item documentation
- Use `//!` for module documentation
- Use `//` for inline comments
- Keep comments concise and clear

```rust
/// Represents a GPU device with monitoring capabilities.
///
/// # Examples
///
/// ```
/// let gpu = Gpu::new(0, "NVIDIA GeForce RTX 3080");
/// assert_eq!(gpu.index(), 0);
/// ```
pub struct Gpu {
    /// The GPU index
    index: u32,
    
    /// The GPU name
    name: String,
}
```

### TypeScript Comments

- Use JSDoc for public functions
- Use `//` for inline comments
- Explain "why" not "what"
- Keep comments up to date

```typescript
/**
 * Gets GPU data for the specified GPU ID.
 * 
 * @param gpuId - The GPU identifier
 * @returns The GPU data
 */
export const getGpuData = async (gpuId: string): Promise<GpuData> => {
  // Fetch data from Tauri backend
  return await invoke<GpuData>('gpu_get_data', { gpuId });
};
```

### Comment Guidelines

- **Do**: Explain intent, not implementation
- **Do**: Keep comments up to date
- **Do**: Use clear, concise language
- **Don't**: State the obvious
- **Don't**: Add comments for simple code
- **Don't**: Add outdated comments

---

## Testing Strategy

### Rust Testing

- Use `#[test]` for unit tests
- Use `#[cfg(test)]` for test modules
- Use `#[should_panic]` for panic tests
- Use `#[tokio::test]` for async tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gpu_creation() {
        let gpu = Gpu::new(0, "NVIDIA GeForce RTX 3080");
        assert_eq!(gpu.index(), 0);
        assert_eq!(gpu.name(), "NVIDIA GeForce RTX 3080");
    }
    
    #[tokio::test]
    async fn test_monitoring_start() {
        let result = start_monitoring("gpu-0").await;
        assert!(result.is_ok());
    }
}
```

### TypeScript Testing

- Use `vitest` for testing
- Use `@testing-library/react` for component testing
- Mock Tauri commands
- Test all public APIs

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FanControlPanel } from './FanControlPanel';

describe('FanControlPanel', () => {
  it('renders fan control with correct value', () => {
    render(<FanControlPanel gpuId="gpu-0" currentSpeed={50} />);
    expect(screen.getByRole('slider')).toHaveValue(50);
  });
  
  it('calls onChange when slider value changes', () => {
    const handleChange = vi.fn();
    render(<FanControlPanel gpuId="gpu-0" currentSpeed={50} onChange={handleChange} />);
    
    // Simulate slider change
    // ...
    
    expect(handleChange).toHaveBeenCalledWith(75);
  });
});
```

### Test Coverage

- **Unit Tests**: All public functions
- **Integration Tests**: Key workflows
- **E2E Tests**: Critical user journeys
- **Target**: 80%+ coverage

---

## Git Workflow

### Branching Strategy

```
main
├── release/v1.0.0
│   └── hotfix/fix-bug-123
├── develop
│   ├── feature/new-monitoring
│   ├── feature/overlay
│   └── bugfix/crash-on-startup
```

### Branch Naming

| Branch Type | Convention | Example |
|-------------|-----------|---------|
| Feature | `feature/` + description | `feature/new-monitoring` |
| Bugfix | `bugfix/` + description | `bugfix/crash-on-startup` |
| Hotfix | `hotfix/` + description | `hotfix/security-patch` |
| Release | `release/` + version | `release/v1.0.0` |
| Develop | `develop` | `develop` |

### Commit Naming

| Type | Convention | Example |
|------|-----------|---------|
| Feature | `feat: ` + description | `feat: add GPU monitoring` |
| Bugfix | `fix: ` + description | `fix: handle GPU not found` |
| Documentation | `docs: ` + description | `docs: update README` |
| Style | `style: ` + description | `style: format code` |
| Refactor | `refactor: ` + description | `refactor: improve monitoring` |
| Test | `test: ` + description | `test: add unit tests` |
| Chore | `chore: ` + description | `chore: update dependencies` |

### Commit Message Format

```
type: description

- Detailed explanation
- Multiple lines allowed
- Reference issues: #123
```

### Pull Request Workflow

1. Create feature branch from `develop`
2. Implement feature with tests
3. Run linting and tests
4. Create pull request
5. Request review
6. Address feedback
7. Merge to `develop`

---

## Pull Request Standards

### PR Checklist

- [ ] Code follows guidelines
- [ ] Tests pass
- [ ] Linting passes
- [ ] Documentation updated
- [ ] CHANGELOG updated
- [ ] Reviewer approved

### PR Guidelines

- **Small PRs**: Keep PRs focused and small
- **Clear Description**: Explain what and why
- **Screenshots**: Add UI changes screenshots
- **Tests**: Include tests for new code
- **Review**: Request review from team members

### PR Template

```
## Description
[Explain the changes]

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Screenshots (if applicable)
[Add screenshots for UI changes]

## References
- Issue: #123
- Design: [link]
```

---

*Last updated: 2024-01-01*
