# macOS EvdevInputManager Implementation

## Overview

This document describes the macOS implementation of `EvdevInputManager` for the Raycast Linux project. The implementation provides a cross-platform input management system that works on both Linux (using evdev) and macOS (using CoreGraphics).

## Implementation Details

### Architecture

The implementation uses conditional compilation to provide platform-specific implementations:

- **Linux**: Uses evdev for low-level input device access and uinput for event injection
- **macOS**: Uses CoreGraphics (Quartz Event Services) for event injection

### Key Features

#### Text Injection
- Both platforms support text injection via clipboard-based operations
- Linux: Uses Ctrl+V for paste operations
- macOS: Uses Cmd+V for paste operations
- Supports backspace sequences for text deletion

#### Key Event Injection
- Support for injecting individual key events (backspace, arrow keys, etc.)
- Platform-specific key code mappings

#### Input Monitoring
- **Linux**: Full keyboard event monitoring using evdev and xkbcommon
- **macOS**: Placeholder implementation (requires accessibility permissions for full functionality)

### macOS Implementation Details

The macOS implementation (`EvdevInputManager` with `#[cfg(target_os = "macos")]`) provides:

1. **Simplified Structure**: Uses a unit struct instead of storing state to avoid thread safety issues
2. **Static Methods**: Key injection methods are static to avoid `Send`/`Sync` requirements
3. **Event Source per Operation**: Creates a new `CGEventSource` for each operation to avoid thread safety concerns
4. **Key Code Mapping**: Maps `EnigoKey` enum values to macOS virtual key codes

### Key Code Mappings

The implementation includes mappings for:
- Letter keys (a-z, A-Z)
- Number keys (0-9) and their shifted symbols
- Special characters and punctuation
- Control keys (Backspace, Left Arrow)

### Usage

The implementation is automatically selected based on the target platform:

```rust
// On macOS, this will use the CoreGraphics implementation
let input_manager = EvdevInputManager::new()?;

// Inject text using clipboard
input_manager.inject_text("Hello, World!")?;

// Inject backspace events
input_manager.inject_key_clicks(EnigoKey::Backspace, 3)?;
```

### Limitations

#### macOS Limitations
1. **Input Monitoring**: The current implementation provides only a placeholder for keyboard monitoring
2. **Accessibility Permissions**: Full functionality would require accessibility permissions
3. **Event Tap**: A complete implementation would need CGEventTap for real-time monitoring

#### Future Improvements
1. Implement proper CGEventTap-based keyboard monitoring
2. Add support for more key types and modifiers
3. Handle accessibility permission requests
4. Optimize event source creation

### Dependencies

The macOS implementation requires these additional dependencies:
- `core-graphics = "0.23"`
- `core-foundation = "0.10"`

### Thread Safety

The implementation is designed to be thread-safe by:
- Using static methods that don't hold state
- Creating new event sources for each operation
- Avoiding shared mutable state

### Integration

The implementation integrates seamlessly with the existing codebase:
- Implements the same `InputManager` trait
- Uses the same error handling patterns
- Maintains compatibility with the existing API

## Testing

The implementation has been tested on macOS and successfully:
- Compiles without errors
- Links with the required frameworks
- Provides text injection functionality
- Maintains thread safety requirements

## Security Considerations

- Text injection requires appropriate permissions on macOS
- The application may need to be granted accessibility permissions for full functionality
- Clipboard operations are temporary and restore original content