// glue.js — the thin renderer between the Rust/WASM engine and the DOM.
//
// The engine (engine.wasm) owns all game logic and produces either a full UI
// tree (on boot) or a list of DOM patches (every tick). This file:
//   - loads and talks to the wasm module (hand-rolled FFI, no wasm-bindgen)
//   - builds/patches the DOM from the render trees
//   - forwards clicks, keys and input values back into the engine
//   - draws the tree canvas, popups and particles
//   - handles localStorage saves, export/import and the options tab

(async function () {
	"use strict";

	// ------------------------------------------------------------------
	// WASM plumbing
	// ------------------------------------------------------------------

	const response = await fetch("engine.wasm");
	const bytes = await response.arrayBuffer();
	const { instance } = await WebAssembly.instantiate(bytes, {});
	const ex = instance.exports;

	const encoder = new TextEncoder();
	const decoder = new TextDecoder();

	function call(command) {
		const cmd = encoder.encode(JSON.stringify(command));
		const ptr = ex.tmt_alloc(cmd.length);
		if (ptr === 0) throw new Error("tmt_alloc failed");
		new Uint8Array(ex.memory.buffer, ptr, cmd.length).set(cmd);
		const len = ex.tmt_call(ptr, cmd.length);
		ex.tmt_free(ptr, cmd.length);
		const outPtr = ex.tmt_result_ptr();
		const out = decoder.decode(new Uint8Array(ex.memory.buffer, outPtr, len));
		return JSON.parse(out);
	}

	// ------------------------------------------------------------------
	// DOM building & patching
	// ------------------------------------------------------------------

	const app = document.getElementById("app");
	const els = new Map(); // id -> element

	function makeElement(node) {
		const el = document.createElement(node.t);
		el.id = node.i;
		if (node.c) el.className = node.c;
		if (node.s) el.style.cssText = node.s;
		if (node.x) el.innerHTML = node.x;
		if (node.a) {
			for (const [k, v] of Object.entries(node.a)) {
				if (k === "value") el.value = v;
				else if (k === "checked") el.checked = v === "true" || v === true;
				else el.setAttribute(k, v);
			}
		}
		if (node.e) el.dataset.ev = node.e;
		els.set(node.i, el);
		for (const child of node.k || []) el.appendChild(makeElement(child));
		return el;
	}

	function forgetElement(el) {
		els.delete(el.id);
		for (const sub of el.querySelectorAll("[id]")) els.delete(sub.id);
	}

	// Resolve an element by id, healing the map when an entry was lost or
	// points at a detached node (patches must never be silently dropped).
	function getEl(id) {
		let el = els.get(id);
		if (el && el.isConnected) return el;
		el = document.getElementById(id);
		if (el) els.set(id, el);
		return el;
	}

	function applyPatch(p) {
		if (p[0] === "a") {
			const el = getEl(p[1]);
			if (!el) return;
			const key = p[2];
			const val = p[3];
			if (key === "class") el.className = val;
			else if (key === "style") el.style.cssText = val;
			else if (key === "html") el.innerHTML = val;
			else if (key === "value") el.value = val;
			else if (key === "checked") el.checked = !!val;
			else if (val === null) el.removeAttribute(key);
			else el.setAttribute(key, val);
		} else if (p[0] === "ins") {
			const parent = getEl(p[1]);
			if (!parent) return;
			const index = p[2];
			const node = p[3];
			let el = getEl(node.i);
			if (el && el.parentElement !== parent) {
				parent.insertBefore(el, parent.children[index] || null);
			} else if (!el) {
				el = makeElement(node);
				parent.insertBefore(el, parent.children[index] || null);
			} else if (el !== parent.children[index]) {
				parent.insertBefore(el, parent.children[index] || null);
			}
		} else if (p[0] === "rm") {
			const el = getEl(p[2]);
			if (el) {
				el.remove();
				forgetElement(el);
			}
		}
	}

	function render(resp) {
		const p = resp.p;
		if (p && p.full) {
			app.innerHTML = "";
			els.clear();
			app.appendChild(makeElement(p.full));
		} else if (Array.isArray(p)) {
			for (const patch of p) applyPatch(patch);
		}
	}

	// ------------------------------------------------------------------
	// State from the engine
	// ------------------------------------------------------------------

	let modId = "modbase";
	let lastOptionsText = "";
	let treeData = null;
	let ticking = true;
	let mouseX = 0;
	let mouseY = 0;

	const THEMES = {
		default: {
			color: "#dfdfdf", points: "#ffffff", locked: "#bf8f8f",
			background: "#0f0f0f", background_tooltip: "rgba(0, 0, 0, 0.75)",
		},
		aqua: {
			color: "#bfdfff", points: "#dfefff", locked: "#c4a7b3",
			background: "#001f3f", background_tooltip: "rgba(0, 15, 31, 0.75)",
		},
	};

	function applyTheme(name) {
		const theme = THEMES[name] || THEMES.default;
		const body = document.body;
		body.style.setProperty("--background", theme.background);
		body.style.setProperty("--background_tooltip", theme.background_tooltip);
		body.style.setProperty("--color", theme.color);
		body.style.setProperty("--points", theme.points);
		body.style.setProperty("--locked", theme.locked);
	}

	function applyHq(on) {
		const body = document.body;
		body.style.setProperty("--hqProperty1", on ? "2px solid" : "4px solid");
		body.style.setProperty("--hqProperty2a", on ? "-4px -4px 4px rgba(0, 0, 0, 0.25) inset" : "-4px -4px 4px rgba(0, 0, 0, 0) inset");
		body.style.setProperty("--hqProperty2b", on ? "0px 0px 20px var(--background)" : "");
		body.style.setProperty("--hqProperty3", on ? "2px 2px 4px rgba(0, 0, 0, 0.25)" : "none");
	}

	// ------------------------------------------------------------------
	// Response handling
	// ------------------------------------------------------------------

	function handleResponse(resp) {
		if (resp.error) console.error("engine:", resp.error);

		if (resp.confirm) {
			const ok = confirm(resp.confirm);
			if (ok) {
				if (resp.ev === "__import__") {
					handleResponse(call({ op: "import", data: resp.data, forced: true }));
				} else {
					handleResponse(call({ op: "event", id: resp.ev, payload: { confirmed: true } }));
				}
			}
			return;
		}

		if (resp.title) document.title = resp.title;
		if (resp.modId) modId = resp.modId;

		render(resp);

		if (resp.theme) applyTheme(resp.theme);
		if (typeof resp.hq === "boolean") applyHq(resp.hq);

		if (resp.opts) {
			const text = JSON.stringify(resp.opts);
			if (text !== lastOptionsText) {
				lastOptionsText = text;
				try {
					localStorage.setItem(modId + "_options", btoa(unescape(encodeURIComponent(text))));
				} catch (e) { /* storage full/unavailable */ }
			}
		}

		if (resp.save) {
			try {
				localStorage.setItem(modId, btoa(unescape(encodeURIComponent(resp.save))));
			} catch (e) { /* ignore */ }
		}

		if (resp.tree) treeData = resp.tree;

		if (resp.pop) for (const popup of resp.pop) showPopup(popup);
		if (resp.particles) for (const p of resp.particles) spawnParticles(p);

		if (resp.nanAlert) {
			ticking = false;
			alert("Invalid value found in player, named '" + resp.nanAlert +
				"'. Please let the creator of this mod know! You can refresh the page, and you will be un-NaNed.");
		}
	}

	// ------------------------------------------------------------------
	// Events -> engine
	// ------------------------------------------------------------------

	function sendEvent(id, payload) {
		handleResponse(call({ op: "event", id, payload: payload || {} }));
	}

	document.addEventListener("click", (e) => {
		const target = e.target.closest("[data-ev],[data-tmt]");
		if (!target) return;
		const id = target.dataset.ev || target.dataset.tmt;
		if (!id) return;
		if (id.startsWith("js:")) {
			handleJsEvent(id);
			return;
		}
		sendEvent(id, { shift: e.shiftKey });
	});

	document.addEventListener("change", (e) => {
		const target = e.target.closest("[data-ev],[data-tmt]");
		if (!target) return;
		const id = target.dataset.ev || target.dataset.tmt;
		if (!id || id.startsWith("js:")) return;
		sendEvent(id, { value: target.value });
	});

	// Live slider feedback.
	document.addEventListener("input", (e) => {
		if (e.target.type !== "range") return;
		const target = e.target.closest("[data-ev],[data-tmt]");
		if (!target) return;
		const id = target.dataset.ev || target.dataset.tmt;
		if (!id || id.startsWith("js:")) return;
		sendEvent(id, { value: target.value });
	});

	// Hold-to-repeat (buyables, clickables with onHold, grid).
	document.addEventListener("mousedown", (e) => {
		const target = e.target.closest("[data-hold]");
		if (!target || !target.dataset.ev) return;
		const id = holdEventId(target.dataset.ev);
		let ticks = 0;
		const interval = setInterval(() => {
			ticks++;
			if (ticks >= 5) sendEvent(id);
		}, 50);
		const stop = () => {
			clearInterval(interval);
			document.removeEventListener("mouseup", stop);
			document.removeEventListener("mouseleave", stop);
		};
		document.addEventListener("mouseup", stop);
		document.addEventListener("mouseleave", stop);
	});

	function holdEventId(ev) {
		if (ev.startsWith("click:")) return "hold" + ev;
		if (ev.startsWith("grid:")) return "hold" + ev;
		return ev;
	}

	document.addEventListener("keydown", (e) => {
		const tag = document.activeElement && document.activeElement.tagName;
		if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
		let key = e.key;
		if (e.ctrlKey) key = "ctrl+" + key;
		if (e.ctrlKey) e.preventDefault();
		sendEvent("key:" + key);
	});

	document.addEventListener("mousemove", (e) => {
		mouseX = e.clientX;
		mouseY = e.clientY;
	});

	function handleJsEvent(id) {
		if (id === "js:export") {
			const resp = call({ op: "export" });
			const text = btoa(resp.save);
			if (navigator.clipboard && navigator.clipboard.writeText) {
				navigator.clipboard.writeText(text).then(
					() => alert("Save copied to clipboard!"),
					() => prompt("Copy your save:", text)
				);
			} else {
				prompt("Copy your save:", text);
			}
		} else if (id === "js:import") {
			const input = prompt("Paste your save here");
			if (input === null || input.trim() === "") return;
			try {
				const data = decodeURIComponent(escape(atob(input.trim())));
				handleResponse(call({ op: "import", data }));
			} catch (err) {
				alert("Invalid save file.");
			}
		}
	}

	// Exposed for raw-html content (the demo's particle button).
	window.TMT = {
		particles(kind) { sendEvent("particles:" + kind); },
	};

	// ------------------------------------------------------------------
	// Popups
	// ------------------------------------------------------------------

	const popupContainer = document.createElement("div");
	popupContainer.className = "popup-container";
	document.body.appendChild(popupContainer);

	function showPopup(popup) {
		const el = document.createElement("div");
		el.className = "popup " + (popup.type || "default-popup").replace("-popup", "") + "-popup";
		if (popup.color) el.style.backgroundColor = popup.color;
		el.innerHTML = "<h3></h3><br><h2></h2>";
		el.querySelector("h3").textContent = popup.title || "";
		el.querySelector("h2").innerHTML = popup.message || "";
		el.addEventListener("click", () => el.remove());
		popupContainer.appendChild(el);
		setTimeout(() => el.remove(), 3000);
	}

	// ------------------------------------------------------------------
	// Particles (minimal port of js/technical/particleSystem.js)
	// ------------------------------------------------------------------

	const particleLayer = document.createElement("div");
	particleLayer.className = "particle-layer";
	document.body.appendChild(particleLayer);

	const particles = [];

	function spawnParticles(p) {
		const spec = p.spec || {};
		for (let i = 0; i < p.count; i++) {
			const dir = (spec.spread || 0) * (i - p.count / 2 + 0.5);
			const x = p.x + Math.sin(dir * Math.PI / 180) * (spec.offset || 0);
			const y = p.y - Math.cos(dir * Math.PI / 180) * (spec.offset || 0);
			particles.push({
				x, y,
				dir,
				speed: spec.speed || 0,
				xVel: (spec.speed || 0) * Math.sin(dir * Math.PI / 180),
				yVel: -(spec.speed || 0) * Math.cos(dir * Math.PI / 180),
				gravity: spec.gravity || 0,
				rotation: spec.rotation || 0,
				angle: 0,
				time: spec.time || 3,
				fadeInTime: spec.fadeInTime || 0,
				fadeInTimer: spec.fadeInTime || 0,
				kind: p.kind,
				content: p.content,
				el: null,
			});
		}
	}

	function updateParticles(diff) {
		for (const p of particles) {
			p.time -= diff;
			p.fadeInTimer -= diff;
			p.angle += p.rotation * diff;
			p.x += p.xVel * diff;
			p.y += p.yVel * diff;
			p.yVel += p.gravity * diff;
		}
		for (let i = particles.length - 1; i >= 0; i--) {
			if (particles[i].time < 0) {
				if (particles[i].el) particles[i].el.remove();
				particles.splice(i, 1);
			}
		}
		for (const p of particles) {
			if (!p.el) {
				p.el = document.createElement("div");
				p.el.className = "particle";
				if (p.kind === "text") p.el.innerHTML = p.content;
				else p.el.style.backgroundImage = "url(" + p.content + ")";
				particleLayer.appendChild(p.el);
			}
			let opacity = 1;
			if (p.time < 1 && p.time > 0) opacity = p.time;
			if (p.fadeInTimer > 0) opacity = 1 - p.fadeInTimer / (p.fadeInTime || 1);
			p.el.style.left = (p.x - 17) + "px";
			p.el.style.top = (p.y - 17) + "px";
			p.el.style.width = "35px";
			p.el.style.height = "35px";
			p.el.style.backgroundSize = "contain";
			p.el.style.transform = "rotate(" + p.angle + "deg)";
			p.el.style.opacity = Math.max(0, Math.min(1, opacity));
		}
	}

	// ------------------------------------------------------------------
	// Tree canvas
	// ------------------------------------------------------------------

	function drawTree() {
		const canvas = els.get("treeCanvas");
		if (!canvas || !treeData) return;
		canvas.width = 0;
		canvas.height = 0;
		canvas.width = window.innerWidth;
		canvas.height = window.innerHeight;
		const ctx = canvas.getContext("2d");
		ctx.clearRect(0, 0, canvas.width, canvas.height);
		for (const b of treeData.branches || []) {
			const [fromId, toId, color, width] = b;
			const from = els.get(fromId) || document.getElementById(fromId);
			const to = els.get(toId) || document.getElementById(toId);
			if (!from || !to) continue;
			const r1 = from.getBoundingClientRect();
			const r2 = to.getBoundingClientRect();
			const x1 = r1.left + r1.width / 2 + window.scrollX;
			const y1 = r1.top + r1.height / 2 + window.scrollY;
			const x2 = r2.left + r2.width / 2 + window.scrollX;
			const y2 = r2.top + r2.height / 2 + window.scrollY;
			ctx.lineWidth = width;
			ctx.strokeStyle = color;
			ctx.beginPath();
			ctx.moveTo(x1, y1);
			ctx.lineTo(x2, y2);
			ctx.stroke();
		}
	}

	window.addEventListener("resize", drawTree);
	setInterval(drawTree, 500);

	// ------------------------------------------------------------------
	// Boot & main loop
	// ------------------------------------------------------------------

	function readLocalStorage(key) {
		try {
			const raw = localStorage.getItem(key);
			if (raw === null || raw === undefined) return null;
			return decodeURIComponent(escape(atob(raw)));
		} catch (e) {
			return null;
		}
	}

	const bootResp = call({
		op: "init",
		now: Date.now(),
		w: window.innerWidth,
		h: window.innerHeight,
		save: readLocalStorage("modbase"),
		options: readLocalStorage("modbase_options"),
	});
	handleResponse(bootResp);

	setInterval(() => {
		if (!ticking) return;
		const treeTab = els.get("treeTab");
		const resp = call({
			op: "tick",
			now: Date.now(),
			w: window.innerWidth,
			h: window.innerHeight,
			scrolled: !!(treeTab && treeTab.scrollTop > 30),
			mx: mouseX,
			my: mouseY,
		});
		handleResponse(resp);
		updateParticles(0.05);
	}, 50);

	window.addEventListener("beforeunload", () => {
		// Synchronous wasm call: persist the current state.
		try {
			const resp = call({ op: "export" });
			localStorage.setItem(modId, btoa(unescape(encodeURIComponent(resp.save))));
		} catch (e) { /* best effort */ }
	});
})();
