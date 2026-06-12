#!/bin/bash
set -e

# Step 1: Add wl_output_objects field to WaylandClientState
sed -i '/^pub(crate) struct WaylandClientState {$/a\    wl_output_objects: HashMap<ObjectId, wl_output::WlOutput>,' \
    src/platform/linux/wayland/client.rs

# Step 2: Initialize wl_output_objects in the struct creation
sed -i '/outputs: HashMap::default(),$/a\            wl_output_objects: HashMap::default(),' \
    src/platform/linux/wayland/client.rs

# Step 3: Store WlOutput when output is created (after binding)
# Find the line where output is bound and store it
sed -i '/let output = globals.registry().bind::<wl_output::WlOutput, _, _>($/,/)$/{
    /);$/a\                        state.wl_output_objects.insert(output.id(), output.clone());
}' src/platform/linux/wayland/client.rs

# Step 4: Store WlOutput when new output is registered
sed -i '/let output = registry.bind::<wl_output::WlOutput, _, _>($/,/)$/{
    /);$/a\                    state.wl_output_objects.insert(output.id(), output.clone());
}' src/platform/linux/wayland/client.rs

echo "Patch script created successfully"
