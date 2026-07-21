# Structural Pane Focus Resolution

When moving focus between panes in a split layout (e.g. `Ctrl+B <arrow>`), focus resolution uses structural tree traversal rather than geometric spatial memory. If a target direction contains multiple panes (an asymmetric split), focus simply resolves to the structurally first pane (top-most or left-most) in that branch. We chose this over tmux-style spatial/history tracking because it significantly simplifies the layout engine logic while remaining highly predictable.
