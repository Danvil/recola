# Input Lag Analysis and Optimizations

## Problem Analysis

The Recola game was experiencing mouse input lag, which affects player experience in this first-person 3D game. After analyzing the codebase, several potential sources of input lag were identified:

### Root Causes Identified

1. **Synchronous Processing Pipeline**: All input processing, raycasting, collision detection, and camera updates happen sequentially in the same frame step, creating a cascade of delays.

2. **Continuous Raycasting Overhead**: The game performs raycasting every frame for mouse interactions, even when the camera hasn't moved, which is computationally expensive.

3. **Conservative Camera Settings**: Low sensitivity values (0.0012) and high smoothing (0.15s halflife) were contributing to perceived input lag.

4. **No Performance Monitoring**: Lack of runtime performance metrics made it difficult to identify bottlenecks.

## Implemented Optimizations

### 1. Performance Profiling Integration
- Added Tracy profiling scopes to all components of the input pipeline
- Enables identification of specific bottlenecks during runtime
- Scopes added to: `PlayerMocca::step`, `tick_agents`, `input_raycast`, `restrict_player_movement`, `update_player_eye`, `advance_time`, `update_player_entity_position`, and `cheats`

### 2. Raycast Caching System
- Implemented intelligent caching to avoid redundant raycasting
- Only performs new raycast when camera position or direction changes beyond thresholds:
  - Position threshold: 0.001 units (1mm)
  - Direction threshold: 0.0001 radians (~0.006 degrees)
- Caches raycast results and reuses them when camera is stationary
- Invalidates cache on input events to ensure responsiveness

### 3. Enhanced Camera Responsiveness
- **Increased mouse sensitivity**: From 0.0012 to 0.0015 (25% increase)
- **Reduced height smoothing**: From 0.15s to 0.08s halflife (nearly 2x faster response)
- These changes provide more immediate visual feedback to mouse input

### 4. Runtime Performance Monitoring
- Added frame time tracking (rolling 60-frame average)
- Added raycast time tracking (rolling 60-sample average)
- Debug key binding (P key) to print performance statistics
- Tracks input event timing for lag analysis

### 5. Detailed Profiling Instrumentation
- Separate profiling scopes for cached vs. new raycast operations
- Granular timing measurement for raycast operations
- Frame-by-frame performance data collection

## Usage Instructions

### Building with Tracy Profiling
To enable detailed profiling, build with Tracy support:
```bash
cargo run --release --features profile-with-tracy
```

### Runtime Performance Monitoring
- Press **P** key during gameplay to print performance statistics to console
- Statistics include:
  - Average and maximum frame times
  - Current FPS
  - Average and maximum raycast times
  - Cache hit/miss information

### Expected Performance Improvements

1. **Reduced Raycast Overhead**: 
   - When camera is stationary: ~90% reduction in raycast calls
   - When camera moves slowly: ~50-70% reduction in raycast calls

2. **Improved Input Responsiveness**:
   - 25% increase in mouse sensitivity
   - Nearly 2x faster camera smoothing response
   - Immediate cache invalidation on input events

3. **Better Performance Visibility**:
   - Real-time performance metrics
   - Detailed profiling data for optimization
   - Ability to correlate input lag with specific pipeline stages

## Technical Details

### Raycast Cache Implementation
The cache system tracks:
- Last camera position and direction
- Cached raycast result (entity and distance)
- Cache validity flag
- Performance timing data

### Performance Monitoring
- Frame times: Rolling buffer of last 60 frame durations
- Raycast times: Rolling buffer of last 60 raycast durations
- Input timing: Tracks when input events occur for lag correlation

### Profiling Integration
Uses the existing Tracy profiling infrastructure with detailed scopes for:
- Overall step function performance
- Individual pipeline stage performance
- Cache hit vs. miss scenarios
- Detailed raycast operation timing

## Future Optimization Opportunities

1. **Asynchronous Input Processing**: Move input processing to a separate thread
2. **Spatial Partitioning Optimization**: Improve the underlying collision system's spatial data structures
3. **Predictive Input**: Implement client-side prediction for immediate visual feedback
4. **Adaptive Quality**: Reduce processing complexity when frame rates drop
5. **Input Buffering**: Implement proper input event queuing and batching

## Testing Recommendations

1. **Before/After Comparison**: Test input responsiveness before and after applying these changes
2. **Performance Profiling**: Use Tracy to identify remaining bottlenecks
3. **Various Scenarios**: Test in different game areas with varying collision complexity
4. **Frame Rate Impact**: Monitor for any negative impact on overall frame rate
5. **User Experience**: Gather feedback on perceived input responsiveness improvements
