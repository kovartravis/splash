---
status: accepted
---

# Embedded MCP Server and Pane Layouts

To allow harness agents to programmatically control the TUI (issue 19), we decided to embed an unsecured MCP HTTP/SSE server directly into the Splash process on a dynamic local TCP port, and to evolve the UI layout from simple single-view Tabs into a nested tree of Tabs and Panes.

We chose to embed an MCP server rather than exposing a CLI tool because harnesses natively support structured MCP tool-calling schemas, which is significantly more robust than having an LLM generate bespoke CLI commands and parse stdout. 

We chose to support panes (horizontal and vertical splits) within tabs so that harnesses can display files side-by-side with their own terminal during review phases. This fundamentally changes the domain model from strictly categorized "Harness Tabs" and "File Tabs" to generic "Tabs" that represent a layout of one or more "Panes".
