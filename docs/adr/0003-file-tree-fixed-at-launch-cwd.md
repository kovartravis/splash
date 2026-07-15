# File tree is fixed at the launch working tree; it does not follow the harness CWD

Agent CLIs frequently change directory as they work. The file tree stays rooted at the directory Splash was launched from rather than tracking the active harness's CWD. This keeps the file tree as a stable project-level anchor — the user's frame of reference — rather than an unpredictable moving view that shifts whenever the agent navigates. Users who want to see a subdirectory can open files from the tree directly.
