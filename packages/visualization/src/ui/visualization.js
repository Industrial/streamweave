// StreamWeave Pipeline Visualization JavaScript
// This script handles graph rendering, zoom/pan, node interactions, and real-time metrics

let dagData = null;
let nodeMetrics = new Map(); // Map of node_id -> metrics snapshot
let bottlenecks = new Set(); // Set of node IDs that are bottlenecks
let selectedNodeId = null;
let zoomLevel = 1;
let panX = 0;
let panY = 0;
let isPanning = false;
let lastPanPoint = { x: 0, y: 0 };
let metricsUpdateInterval = null;
let animationFrameId = null;

const svg = document.getElementById("graph-svg");
const container = document.getElementById("graph-container");
const sidebar = document.getElementById("sidebar");
const nodeDetailsDiv = document.getElementById("node-details");
const statusDiv = document.getElementById("status");

// Zoom and pan handlers
svg.addEventListener("wheel", (e) => {
	e.preventDefault();
	const delta = e.deltaY > 0 ? 0.9 : 1.1;
	zoom(delta, e.offsetX, e.offsetY);
});

svg.addEventListener("mousedown", (e) => {
	if (e.button === 0) {
		isPanning = true;
		lastPanPoint = { x: e.clientX - panX, y: e.clientY - panY };
	}
});

svg.addEventListener("mousemove", (e) => {
	if (isPanning) {
		panX = e.clientX - lastPanPoint.x;
		panY = e.clientY - lastPanPoint.y;
		updateTransform();
	}
});

svg.addEventListener("mouseup", () => {
	isPanning = false;
});

svg.addEventListener("mouseleave", () => {
	isPanning = false;
});

function zoom(factor, centerX, centerY) {
	zoomLevel *= factor;
	zoomLevel = Math.max(0.1, Math.min(5, zoomLevel));
	updateTransform();
	updateStatus(`Zoom: ${Math.round(zoomLevel * 100)}%`);
}

function updateTransform() {
	const g = svg.querySelector("g.graph-content");
	if (g) {
		g.setAttribute(
			"transform",
			`translate(${panX}, ${panY}) scale(${zoomLevel})`,
		);
	}
}

function resetZoom() {
	zoomLevel = 1;
	panX = 0;
	panY = 0;
	updateTransform();
	fitToScreen();
	updateStatus("Zoom reset");
}

function fitToScreen() {
	if (!dagData || dagData.nodes.length === 0) return;

	const bbox = svg.getBBox();
	const containerWidth = container.clientWidth;
	const containerHeight = container.clientHeight;

	const scaleX = containerWidth / (bbox.width || 1);
	const scaleY = containerHeight / (bbox.height || 1);
	zoomLevel = Math.min(scaleX, scaleY) * 0.9;

	panX = (containerWidth - bbox.width * zoomLevel) / 2 - bbox.x * zoomLevel;
	panY = (containerHeight - bbox.height * zoomLevel) / 2 - bbox.y * zoomLevel;

	updateTransform();
	updateStatus("Fitted to screen");
}

function updateStatus(message) {
	statusDiv.textContent = message;
	setTimeout(() => {
		statusDiv.textContent = "Ready";
	}, 2000);
}

function exportJSON() {
	if (!dagData) {
		alert("No DAG data loaded");
		return;
	}

	const json = JSON.stringify(dagData, null, 2);
	const blob = new Blob([json], { type: "application/json" });
	const url = URL.createObjectURL(blob);
	const a = document.createElement("a");
	a.href = url;
	a.download = "pipeline-dag.json";
	a.click();
	URL.revokeObjectURL(url);
	updateStatus("JSON exported");
}

function exportDOT() {
	if (!dagData) {
		alert("No DAG data loaded");
		return;
	}

	// Generate DOT format
	let dot = "digraph PipelineDag {\n";
	dot += "  rankdir=LR;\n";
	dot += "  node [shape=box, style=rounded];\n\n";

	dagData.nodes.forEach((node) => {
		const nodeId = sanitizeId(node.id);
		const label = escapeDot(node.metadata.component_type);
		const color = getNodeColor(node.kind);
		dot += `  ${nodeId} [label="${label}", fillcolor=${color}, style="rounded,filled"];\n`;
	});

	dot += "\n";

	dagData.edges.forEach((edge) => {
		const fromId = sanitizeId(edge.from);
		const toId = sanitizeId(edge.to);
		const label = edge.label ? ` [label="${escapeDot(edge.label)}"]` : "";
		dot += `  ${fromId} -> ${toId}${label};\n`;
	});

	dot += "}\n";

	const blob = new Blob([dot], { type: "text/plain" });
	const url = URL.createObjectURL(blob);
	const a = document.createElement("a");
	a.href = url;
	a.download = "pipeline-dag.dot";
	a.click();
	URL.revokeObjectURL(url);
	updateStatus("DOT exported");
}

function sanitizeId(id) {
	return id.replace(/[^a-zA-Z0-9_]/g, "_");
}

function escapeDot(str) {
	return str.replace(/"/g, '\\"').replace(/\n/g, "\\n");
}

function getNodeColor(kind) {
	switch (kind) {
		case "producer":
			return "lightblue";
		case "transformer":
			return "lightgreen";
		case "consumer":
			return "lightcoral";
		default:
			return "lightgray";
	}
}

function showNodeDetails(node) {
	selectedNodeId = node.id;
	sidebar.classList.add("visible");

	// Get real-time metrics if available
	const metrics = nodeMetrics.get(node.id);

	let html = "";
	html += `<div class="node-detail"><label>ID</label><value>${escapeHtml(node.id)}</value></div>`;
	html += `<div class="node-detail"><label>Kind</label><value>${escapeHtml(node.kind)}</value></div>`;
	html += `<div class="node-detail"><label>Component Type</label><value>${escapeHtml(node.metadata.component_type)}</value></div>`;

	if (node.metadata.name) {
		html += `<div class="node-detail"><label>Name</label><value>${escapeHtml(node.metadata.name)}</value></div>`;
	}

	// Add real-time metrics if available
	if (metrics) {
		html += `<div class="node-detail"><label>Throughput</label><value>${metrics.throughput.toFixed(2)} items/s</value></div>`;
		html += `<div class="node-detail"><label>Total Items</label><value>${metrics.total_items}</value></div>`;
		html += `<div class="node-detail"><label>Avg Latency</label><value>${metrics.avg_latency_ms.toFixed(2)} ms</value></div>`;
		html += `<div class="node-detail"><label>P95 Latency</label><value>${metrics.p95_latency_ms.toFixed(2)} ms</value></div>`;
		if (metrics.error_count > 0) {
			html += `<div class="node-detail"><label>Errors</label><value style="color: #ff4444;">${metrics.error_count}</value></div>`;
		}
	}

	if (node.metadata.input_type) {
		html += `<div class="node-detail"><label>Input Type</label><value>${escapeHtml(node.metadata.input_type)}</value></div>`;
	}

	if (node.metadata.output_type) {
		html += `<div class="node-detail"><label>Output Type</label><value>${escapeHtml(node.metadata.output_type)}</value></div>`;
	}

	html += `<div class="node-detail"><label>Error Strategy</label><value>${escapeHtml(node.metadata.error_strategy)}</value></div>`;

	// Highlight if bottleneck
	if (bottlenecks.has(node.id)) {
		html += `<div class="node-detail"><label>Status</label><value style="color: #ff4444;">⚠️ Bottleneck</value></div>`;
	}

	nodeDetailsDiv.innerHTML = html;
}

function hideNodeDetails() {
	selectedNodeId = null;
	sidebar.classList.remove("visible");
}

function escapeHtml(str) {
	const div = document.createElement("div");
	div.textContent = str;
	return div.innerHTML;
}

function initializeVisualization() {
	// Try to load DAG data from URL parameter or default location
	const urlParams = new URLSearchParams(window.location.search);
	const dagUrl = urlParams.get("dag") || "/api/dag";

	fetch(dagUrl)
		.then((response) => {
			if (!response.ok) {
				throw new Error(`Failed to load DAG: ${response.statusText}`);
			}
			return response.json();
		})
		.then((data) => {
			dagData = data;
			renderGraph();
			updateStatus("DAG loaded successfully");

			// Check for embedded metrics data
			if (window.embeddedMetrics) {
				updateMetricsFromData(window.embeddedMetrics);
			}
		})
		.catch((error) => {
			console.error("Error loading DAG:", error);
			updateStatus("Error: " + error.message);
			// Render empty graph as fallback
			renderGraph();
		});
}

function renderGraph() {
	if (!dagData) {
		svg.innerHTML =
			'<text x="50%" y="50%" text-anchor="middle" fill="#888">No DAG data available</text>';
		return;
	}

	// Clear previous content
	svg.innerHTML = `
        <defs>
            <marker id="arrowhead" markerWidth="10" markerHeight="10" 
                    refX="9" refY="3" orient="auto">
                <polygon points="0 0, 10 3, 0 6" fill="#666" />
            </marker>
        </defs>
        <g class="graph-content"></g>
    `;

	const g = svg.querySelector("g.graph-content");

	// Simple layout: horizontal flow
	const nodeSpacing = 200;
	const nodeWidth = 150;
	const nodeHeight = 80;
	let x = 50;
	const centerY = container.clientHeight / 2;

	// Create nodes
	const nodeMap = new Map();
	dagData.nodes.forEach((node, index) => {
		const nodeY = centerY + (index - dagData.nodes.length / 2) * 120;

		const nodeGroup = document.createElementNS(
			"http://www.w3.org/2000/svg",
			"g",
		);
		nodeGroup.setAttribute("class", "node");
		nodeGroup.setAttribute("transform", `translate(${x}, ${nodeY})`);
		nodeGroup.setAttribute("data-node-id", node.id);

		const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
		rect.setAttribute("width", nodeWidth);
		rect.setAttribute("height", nodeHeight);
		rect.setAttribute("rx", 8);
		rect.setAttribute("class", `node-${node.kind}`);
		nodeGroup.appendChild(rect);

		const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
		text.setAttribute("x", nodeWidth / 2);
		text.setAttribute("y", nodeHeight / 2 - 8);
		text.setAttribute("text-anchor", "middle");
		text.setAttribute("dominant-baseline", "middle");
		text.setAttribute("fill", "#000");
		text.setAttribute("font-size", "12");
		text.setAttribute("font-weight", "bold");
		text.setAttribute("data-node-name", node.id);
		text.textContent =
			node.metadata.component_type.split("::").pop() || node.id;
		nodeGroup.appendChild(text);

		// Add throughput text (will be updated by real-time metrics)
		const throughputText = document.createElementNS(
			"http://www.w3.org/2000/svg",
			"text",
		);
		throughputText.setAttribute("x", nodeWidth / 2);
		throughputText.setAttribute("y", nodeHeight / 2 + 12);
		throughputText.setAttribute("text-anchor", "middle");
		throughputText.setAttribute("dominant-baseline", "middle");
		throughputText.setAttribute("fill", "#333");
		throughputText.setAttribute("font-size", "10");
		throughputText.setAttribute("data-node-throughput", node.id);
		throughputText.textContent = "0 items/s";
		nodeGroup.appendChild(throughputText);

		nodeGroup.addEventListener("click", () => {
			// Deselect previous node
			document.querySelectorAll(".node.selected").forEach((n) => {
				n.classList.remove("selected");
			});
			nodeGroup.classList.add("selected");
			showNodeDetails(node);
		});

		g.appendChild(nodeGroup);
		nodeMap.set(node.id, { x: x + nodeWidth / 2, y: nodeY + nodeHeight / 2 });

		x += nodeSpacing;
	});

	// Create edges
	dagData.edges.forEach((edge) => {
		const fromNode = nodeMap.get(edge.from);
		const toNode = nodeMap.get(edge.to);

		if (!fromNode || !toNode) return;

		const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
		const dx = toNode.x - fromNode.x;
		const dy = toNode.y - fromNode.y;
		const controlOffset = Math.abs(dx) * 0.5;

		const pathData = `M ${fromNode.x} ${fromNode.y} C ${fromNode.x + controlOffset} ${fromNode.y}, ${toNode.x - controlOffset} ${toNode.y}, ${toNode.x} ${toNode.y}`;
		path.setAttribute("d", pathData);
		path.setAttribute("class", "edge");
		path.setAttribute("data-edge-id", `${edge.from}-${edge.to}`);
		path.setAttribute("stroke-dasharray", "5,5");
		path.setAttribute("stroke-dashoffset", "0");

		// Add animated marker for data flow
		const marker = document.createElementNS(
			"http://www.w3.org/2000/svg",
			"circle",
		);
		marker.setAttribute("r", "4");
		marker.setAttribute("fill", "#4a9eff");
		marker.setAttribute("opacity", "0");
		marker.setAttribute("data-flow-marker", `${edge.from}-${edge.to}`);
		g.appendChild(marker);

		if (edge.label) {
			const label = document.createElementNS(
				"http://www.w3.org/2000/svg",
				"text",
			);
			label.setAttribute("x", (fromNode.x + toNode.x) / 2);
			label.setAttribute("y", (fromNode.y + toNode.y) / 2 - 5);
			label.setAttribute("text-anchor", "middle");
			label.setAttribute("fill", "#888");
			label.setAttribute("font-size", "10");
			label.textContent = edge.label;
			g.appendChild(label);
		}

		g.appendChild(path);
	});

	// Set SVG dimensions
	svg.setAttribute("width", container.clientWidth);
	svg.setAttribute("height", container.clientHeight);

	// Fit to screen initially
	setTimeout(() => fitToScreen(), 100);

	// Start real-time metrics updates if enabled
	startRealTimeUpdates();
}

// Real-time metrics functions
function startRealTimeUpdates() {
	// Check if real-time mode is enabled (via URL parameter or embedded data)
	const urlParams = new URLSearchParams(window.location.search);
	const realtimeEnabled =
		urlParams.get("realtime") === "true" || window.embeddedRealtimeEnabled;

	if (realtimeEnabled) {
		// Start periodic metrics updates
		metricsUpdateInterval = setInterval(() => {
			updateRealTimeMetrics();
		}, 1000); // Update every second

		// Start animation loop for data flow
		animateDataFlow();
		updateStatus("Real-time metrics enabled");
	}
}

function updateRealTimeMetrics() {
	// Try to fetch metrics from API or use embedded data
	const metricsUrl = "/api/metrics";

	fetch(metricsUrl)
		.then((response) => {
			if (!response.ok) {
				// If API not available, try embedded metrics
				if (window.embeddedMetrics) {
					updateMetricsFromData(window.embeddedMetrics);
				}
				return;
			}
			return response.json();
		})
		.then((data) => {
			if (data) {
				updateMetricsFromData(data);
			}
		})
		.catch(() => {
			// Silently fail - metrics API may not be available
			if (window.embeddedMetrics) {
				updateMetricsFromData(window.embeddedMetrics);
			}
		});
}

function updateMetricsFromData(metricsData) {
	// Update metrics map
	if (Array.isArray(metricsData)) {
		metricsData.forEach((snapshot) => {
			nodeMetrics.set(snapshot.node_id, snapshot);
		});
	}

	// Update node displays with throughput
	nodeMetrics.forEach((metrics, nodeId) => {
		const throughputText = svg.querySelector(
			`[data-node-throughput="${nodeId}"]`,
		);
		if (throughputText) {
			const throughput = metrics.throughput || 0;
			throughputText.textContent = `${throughput.toFixed(1)} items/s`;

			// Update color based on throughput
			if (throughput > 0) {
				throughputText.setAttribute("fill", "#00ff00");
			} else {
				throughputText.setAttribute("fill", "#888");
			}
		}
	});

	// Detect and highlight bottlenecks
	detectAndHighlightBottlenecks();
}

function detectAndHighlightBottlenecks() {
	if (!dagData) return;

	bottlenecks.clear();

	dagData.edges.forEach((edge) => {
		const fromMetrics = nodeMetrics.get(edge.from);
		const toMetrics = nodeMetrics.get(edge.to);

		if (fromMetrics && toMetrics) {
			const fromThroughput = fromMetrics.throughput || 0;
			const toThroughput = toMetrics.throughput || 0;

			// If downstream throughput is significantly lower, it's a bottleneck
			if (fromThroughput > 0 && toThroughput < fromThroughput * 0.8) {
				bottlenecks.add(edge.to);
			}
		}
	});

	// Update node visual appearance for bottlenecks
	dagData.nodes.forEach((node) => {
		const nodeGroup = svg.querySelector(`[data-node-id="${node.id}"]`);
		if (nodeGroup) {
			const rect = nodeGroup.querySelector("rect");
			if (bottlenecks.has(node.id)) {
				rect.setAttribute("stroke", "#ff4444");
				rect.setAttribute("stroke-width", "3");
				rect.setAttribute("stroke-dasharray", "5,5");
			} else {
				rect.removeAttribute("stroke");
				rect.removeAttribute("stroke-width");
				rect.removeAttribute("stroke-dasharray");
			}
		}
	});
}

function animateDataFlow() {
	if (!dagData) return;

	// Animate flow markers along edges
	dagData.edges.forEach((edge) => {
		const marker = svg.querySelector(
			`[data-flow-marker="${edge.from}-${edge.to}"]`,
		);
		const path = svg.querySelector(`[data-edge-id="${edge.from}-${edge.to}"]`);

		if (marker && path && nodeMetrics.has(edge.from)) {
			const metrics = nodeMetrics.get(edge.from);
			const throughput = metrics?.throughput || 0;

			if (throughput > 0) {
				// Animate marker along path
				animateMarkerAlongPath(marker, path, throughput);
			} else {
				marker.setAttribute("opacity", "0");
			}
		}
	});

	animationFrameId = requestAnimationFrame(() => animateDataFlow());
}

function animateMarkerAlongPath(marker, path, speed) {
	const pathLength = path.getTotalLength();
	let offset = parseFloat(marker.getAttribute("data-offset") || "0");

	// Speed affects animation rate (higher throughput = faster animation)
	const increment = Math.min(speed * 0.01, 0.05); // Cap at 5% per frame
	offset = (offset + increment) % 1.0;

	if (offset > 0) {
		const point = path.getPointAtLength(offset * pathLength);
		marker.setAttribute("cx", point.x);
		marker.setAttribute("cy", point.y);
		marker.setAttribute("opacity", "1");
		marker.setAttribute("data-offset", offset.toString());
	} else {
		marker.setAttribute("opacity", "0");
	}
}

// Debug mode variables
let debugModeEnabled = false;
let debugState = "running"; // 'running', 'paused', 'stepping'
let breakpoints = new Set();
let debugSnapshots = [];

// Debug mode functions
function toggleDebugMode() {
	debugModeEnabled = !debugModeEnabled;
	const debugPanel = document.getElementById("debug-panel");
	const toggleButton = document.getElementById("debug-toggle");

	if (debugModeEnabled) {
		debugPanel.classList.add("visible");
		toggleButton.textContent = "Disable Debug";
		updateStatus("Debug mode enabled");
		updateBreakpointList();
	} else {
		debugPanel.classList.remove("visible");
		toggleButton.textContent = "Enable Debug";
		updateStatus("Debug mode disabled");
	}
}

function debugResume() {
	debugState = "running";
	updateStatus("Execution resumed");
	updateDebugControls();
}

function debugStep() {
	debugState = "stepping";
	updateStatus("Stepping forward...");
	updateDebugControls();
	// In a real implementation, this would trigger a step forward in the pipeline
}

function debugStepBack() {
	updateStatus("Stepping backward...");
	// In a real implementation, this would restore previous state
}

function debugPause() {
	debugState = "paused";
	updateStatus("Execution paused");
	updateDebugControls();
}

function addBreakpoint() {
	if (!selectedNodeId) {
		alert("Please select a node first to set a breakpoint");
		return;
	}

	breakpoints.add(selectedNodeId);
	updateBreakpointList();
	updateStatus(`Breakpoint added at ${selectedNodeId}`);
}

function removeBreakpoint(nodeId) {
	breakpoints.delete(nodeId);
	updateBreakpointList();
	updateStatus(`Breakpoint removed from ${nodeId}`);
}

function updateBreakpointList() {
	const breakpointList = document.getElementById("breakpoint-list");
	breakpointList.innerHTML = "";

	if (breakpoints.size === 0) {
		breakpointList.innerHTML =
			'<p style="color: #888; font-size: 0.9rem;">No breakpoints set</p>';
		return;
	}

	breakpoints.forEach((nodeId) => {
		const item = document.createElement("div");
		item.className = "breakpoint-item";
		item.innerHTML = `
            <span>${escapeHtml(nodeId)}</span>
            <button onclick="removeBreakpoint('${nodeId}')" style="padding: 0.25rem 0.5rem; font-size: 0.8rem;">Remove</button>
        `;
		breakpointList.appendChild(item);
	});
}

function updateDebugControls() {
	const resumeBtn = document.querySelector(
		'#debug-controls button[onclick="debugResume()"]',
	);
	const stepBtn = document.querySelector(
		'#debug-controls button[onclick="debugStep()"]',
	);
	const pauseBtn = document.querySelector(
		'#debug-controls button[onclick="debugPause()"]',
	);

	if (debugState === "paused" || debugState === "stepping") {
		resumeBtn.disabled = false;
		stepBtn.disabled = false;
		pauseBtn.disabled = true;
	} else {
		resumeBtn.disabled = true;
		stepBtn.disabled = true;
		pauseBtn.disabled = false;
	}
}

function updateDebugSnapshots(snapshots) {
	const snapshotList = document.getElementById("snapshot-list");
	snapshotList.innerHTML = "";

	if (!snapshots || snapshots.length === 0) {
		snapshotList.innerHTML =
			'<p style="color: #888; font-size: 0.9rem;">No snapshots yet</p>';
		return;
	}

	snapshots.forEach((snapshot, index) => {
		const item = document.createElement("div");
		item.className = "node-detail";
		item.innerHTML = `
            <label>Snapshot ${index + 1} at ${escapeHtml(snapshot.node_id)}</label>
            <value style="font-family: monospace; font-size: 0.85rem; white-space: pre-wrap;">${escapeHtml(snapshot.data)}</value>
        `;
		snapshotList.appendChild(item);
	});
}

// Expose debug functions
window.toggleDebugMode = toggleDebugMode;
window.debugResume = debugResume;
window.debugStep = debugStep;
window.debugStepBack = debugStepBack;
window.debugPause = debugPause;
window.addBreakpoint = addBreakpoint;
window.removeBreakpoint = removeBreakpoint;

// Expose functions for button handlers
window.resetZoom = resetZoom;
window.fitToScreen = fitToScreen;
window.exportJSON = exportJSON;
window.exportDOT = exportDOT;
