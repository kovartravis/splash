# The agent terminal is the hero; the file tree and editor are supporting context

Splash's primary mental model is an agent harness shell, not a code editor. The harness terminal owns the main pane by default; files open in tabs alongside it rather than the harness terminal opening in a pane alongside the editor. This shapes every layout and UX priority decision: when in doubt, what serves the running harness wins over what serves the file author.
