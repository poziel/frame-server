# Security

Terminal and agent execution run with the host user's privileges. They are not
an OS sandbox. Expose them only to authorized content. Hosts own authentication,
credential storage, working-directory policy, tool permissions, executable
selection, and the trust boundary between UI requests and native execution.

Never accept a terminal owner identifier directly from untrusted callers.
Close sessions when their host disconnects. Explicitly configure the deployed
Claude helper path and interpreter; a source-checkout default is not a deployed
application path. Inspect provider capabilities before allowing sensitive work.

Report suspected vulnerabilities privately to the maintainer through GitHub
private vulnerability reporting when enabled. Do not post credentials or
private application data in public issues. This initial release has no formal
support or response-time guarantee.
