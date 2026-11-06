#!/usr/bin/env python3
"""
End-to-end pytest suite for the SSQL Launch Website & Interactive Playground.

Run with:
    uv run --frozen pytest
"""

import json
import re
import socket
import sqlite3
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

import pytest
import ssql

REPO_ROOT = Path(__file__).resolve().parent.parent
SCHEMA_PATH = REPO_ROOT / "examples" / "university_bookstore" / "SAMSQL" / "schema.सं"


def _find_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture(scope="module")
def playground_server():
    port = _find_free_port()
    base_url = f"http://127.0.0.1:{port}"
    proc = subprocess.Popen(
        [sys.executable, str(REPO_ROOT / "website" / "server.py"), "--port", str(port)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    # Wait for real readiness. On startup the server rebuilds stale site
    # artifacts (WASM bundle, wheel, downloads), which takes about a minute
    # after a compiler change, so the budget is generous; a server that exits
    # or never answers fails the fixture with its own output instead of
    # surfacing later as "connection refused" in every test.
    deadline = time.time() + 600.0
    ready = False
    while time.time() < deadline:
        if proc.poll() is not None:
            out, err = proc.communicate()
            pytest.fail(f"website/server.py exited with {proc.returncode} before serving:\n{err.decode()}\n{out.decode()}")
        try:
            with urllib.request.urlopen(f"{base_url}/index.html", timeout=1) as resp:
                if resp.status == 200:
                    ready = True
                    break
        except Exception:
            time.sleep(0.2)
    if not ready:
        proc.terminate()
        pytest.fail("website/server.py did not serve index.html within 600 s")
    yield base_url
    proc.terminate()
    proc.wait(timeout=5)


def test_static_and_wasm_assets(playground_server):
    expected_assets = [
        ("/index.html", "text/html"),
        ("/styles.css", "text/css"),
        ("/app.js", "text/javascript"),
        ("/pkg/ssql_wasm.js", "text/javascript"),
        ("/pkg/ssql_wasm_bg.wasm", "application/wasm"),
        ("/data/corpus.json", "application/json"),
        ("/data/content.json", "application/json"),
    ]
    for path, expected_mime in expected_assets:
        with urllib.request.urlopen(f"{playground_server}{path}", timeout=5) as resp:
            assert resp.status == 200
            ctype = resp.headers.get("Content-Type", "")
            assert expected_mime in ctype
            body = resp.read()
            assert len(body) > 100


def test_inline_constitution_and_no_zero_loan_words(playground_server):
    for rel in ("index.html", "app.js", "styles.css"):
        text = (REPO_ROOT / "website" / rel).read_text(encoding="utf-8")
        assert "Zero Loan Words" not in text, f"Found forbidden phrase in {rel}"

    with urllib.request.urlopen(f"{playground_server}/data/content.json", timeout=5) as resp:
        content = json.loads(resp.read().decode("utf-8"))

    constitution = content["constitution"]
    assert len(constitution["articles"]) == 10
    assert constitution["articles"][0]["sanskrit"] == "मूलदृष्टिः"
    assert constitution["articles"][3]["sanskrit"] == "वाचिकता तथा शुद्धता"
    assert constitution["articles"][9]["sanskrit"] == "परिवर्तनस्य विधानम्"

    index_html = (REPO_ROOT / "website" / "index.html").read_text(encoding="utf-8")
    for required_id in (
        'id="constitution"',
        'id="tutorials"',
        'id="downloads"',
        'id="editor-autocomplete"',
        'id="editor-hover-bar"',
        'id="ssql-highlight"',
        'id="ssql-gutter"',
        'id="chip-palette-items"',
    ):
        assert required_id in index_html, f"Missing {required_id} in index.html"


def test_corpus_projects_and_multi_project_seeding(playground_server):
    with urllib.request.urlopen(f"{playground_server}/data/corpus.json", timeout=5) as resp:
        corpus = json.loads(resp.read().decode("utf-8"))

    # One project per example directory with a schema.सं, nothing invented.
    example_count = len(list((REPO_ROOT / "examples").rglob("schema.सं")))
    assert corpus["project_count"] == example_count
    assert corpus["total_queries"] >= 175
    projects = corpus["projects"]
    assert len(projects) == example_count

    # Switch to the Gurukula project via `/api/reset` and run its first query
    gurukula = next(p for p in projects if p["id"] == "gurukula_management/SAMSQL")
    reset_req = urllib.request.Request(
        f"{playground_server}/api/reset",
        data=json.dumps({"project_id": "gurukula_management/SAMSQL"}).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(reset_req, timeout=5) as resp:
        assert json.loads(resp.read().decode("utf-8"))["ok"] is True

    compiler = ssql.Compiler(gurukula["schema_ssql"], "sqlite")
    q_ssql = next(q for q in gurukula["queries"] if q["id"] == "q01")["ssql"]
    compiled = compiler.compile_safe(q_ssql)
    assert compiled.success, f"Gurukula query failed: {compiled.error}"

    exec_req = urllib.request.Request(
        f"{playground_server}/api/execute",
        data=json.dumps({"sql": compiled.sql}).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(exec_req, timeout=5) as resp:
        exec_data = json.loads(resp.read().decode("utf-8"))
        assert exec_data["ok"] is True
        assert len(exec_data["rows"]) == 6  # the six teachers in seed.सं

    # Reset back to default University Bookstore
    reset_default = urllib.request.Request(
        f"{playground_server}/api/reset",
        data=b"{}",
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(reset_default, timeout=5) as resp:
        assert json.loads(resp.read().decode("utf-8"))["ok"] is True


def test_tutorials_and_interactive_exercises_end_to_end(playground_server):
    with urllib.request.urlopen(f"{playground_server}/data/content.json", timeout=5) as resp:
        content = json.loads(resp.read().decode("utf-8"))

    tutorials = content["tutorials"]
    assert len(tutorials) == 10

    schema_text = SCHEMA_PATH.read_text(encoding="utf-8")
    compiler = ssql.Compiler(schema_text, "sqlite")

    for chap in tutorials:
        ex = chap["exercise"]
        res = compiler.compile_safe(ex["solution_ssql"])
        assert res.success, f"Exercise {chap['id']} solution failed to compile: {res.error}"
        assert res.sql is not None

        if "expected_sql_contains" in ex:
            assert ex["expected_sql_contains"] in res.sql
        else:
            exec_req = urllib.request.Request(
                f"{playground_server}/api/execute",
                data=json.dumps({"sql": res.sql}).encode("utf-8"),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(exec_req, timeout=5) as resp:
                exec_data = json.loads(resp.read().decode("utf-8"))
                assert exec_data["ok"] is True, f"Exercise {chap['id']} SQL failed: {exec_data}"
                assert len(exec_data["rows"]) >= ex.get("expected_min_rows", 1)


def test_downloadable_artifacts_built_and_served(playground_server):
    with urllib.request.urlopen(f"{playground_server}/data/content.json", timeout=5) as resp:
        content = json.loads(resp.read().decode("utf-8"))

    downloads = content["downloads"]
    assert len(downloads) == 5
    expected_ids = {"vsix", "compiler", "lsp", "wheel", "wasm"}
    assert {d["id"] for d in downloads} == expected_ids

    for item in downloads:
        with urllib.request.urlopen(f"{playground_server}{item['url']}", timeout=10) as resp:
            assert resp.status == 200
            cdisp = resp.headers.get("Content-Disposition", "")
            assert "attachment" in cdisp
            assert item["filename"] in cdisp
            data = resp.read()
            assert len(data) > 1000, f"Artifact {item['filename']} too small ({len(data)} B)"


def test_compile_with_ssql_py_and_execute_on_server(playground_server):
    schema_text = SCHEMA_PATH.read_text(encoding="utf-8")
    compiler = ssql.Compiler(schema_text, "sqlite")

    scenarios = [
        (
            "filter",
            "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति इति नियमेन शीर्षकम् च मूल्यम् च आह्रियेताम् ।",
        ),
        (
            "join_agg",
            "विक्रयणेभ्यः आपणैः सह विक्रयणस्य आपणसङ्ख्या आपणस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन आपणस्य नाम इति समूहेन आपणस्य नाम च विक्रयणस्य सकलमूल्यस्य योगः च आह्रियेताम् ।",
        ),
        (
            "left_join",
            "पुस्तकेभ्यः समीक्षाभिः सह पुस्तकस्य सङ्ख्या समीक्षायाः पुस्तकसङ्ख्या इत्यनया समा अस्ति इति वाममेलनेन समीक्षायाः सङ्ख्या रिक्ता अस्ति इति नियमेन पुस्तकस्य सङ्ख्या च पुस्तकस्य शीर्षकम् च आह्रियेताम् ।",
        ),
        (
            "having_order",
            "आपणेभ्यः विक्रयणैः सह आपणस्य सङ्ख्या विक्रयणस्य आपणसङ्ख्या इत्यनया समा अस्ति इति मेलनेन आपणस्य नाम इति समूहेन विक्रयणस्य सकलमूल्यस्य योगः ३० इत्यस्मात् अधिकः अस्ति इति समूहनियमेन विक्रयणस्य सकलमूल्यस्य योगः अवरोहः इति क्रमेण आपणस्य नाम च विक्रयणस्य सकलमूल्यस्य योगः आपणराजस्वम् इति आख्या च आह्रियेताम् ।",
        ),
        (
            "window_rank",
            "पुस्तकेभ्यः शीर्षकम् च भाषा च मूल्यम् च श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च आह्रियन्ताम् ।",
        ),
    ]

    # The expected rows are what the same SQL gives on the bookstore's real
    # data, executed independently of the server.
    corpus = json.loads((REPO_ROOT / "website" / "data" / "corpus.json").read_text(encoding="utf-8"))
    bookstore = next(p for p in corpus["projects"] if p["id"] == "university_bookstore/SAMSQL")
    reference_db = sqlite3.connect(":memory:")
    reference_db.executescript(bookstore["setup_sql"])

    for name, ssql_query in scenarios:
        res = compiler.compile_safe(ssql_query)
        assert res.success, f"Scenario {name} failed to compile: {res.error}"
        assert res.sql is not None
        expected_rows = len(reference_db.execute(res.sql).fetchall())
        assert expected_rows > 0, f"Scenario {name} returns no rows on the bookstore data"

        payload = json.dumps({"sql": res.sql}).encode("utf-8")
        req = urllib.request.Request(
            f"{playground_server}/api/execute",
            data=payload,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            assert resp.status == 200
            data = json.loads(resp.read().decode("utf-8"))
            assert data["ok"] is True, f"SQLite execution failed for {name}: {data}"
            assert len(data["rows"]) == expected_rows, f"Scenario {name}: got {len(data['rows'])} rows"

    # Verify /api/reset endpoint
    reset_req = urllib.request.Request(
        f"{playground_server}/api/reset",
        data=b"{}",
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(reset_req, timeout=5) as resp:
        assert resp.status == 200
        assert json.loads(resp.read().decode("utf-8"))["ok"] is True


def test_examples_and_corpus_have_pure_devanagari_and_descriptive_titles(playground_server):
    # 1. Every .सं file under examples/ must contain zero // or /* comments
    for path in sorted((REPO_ROOT / "examples").rglob("*.सं")):
        text = path.read_text(encoding="utf-8")
        assert "//" not in text, f"Found // comment in {path}"
        assert "/*" not in text, f"Found /* comment in {path}"
        assert text.strip(), f"Empty .सं file: {path}"

    # 2. Every project in corpus.json must have comment-free schema, seed, and queries,
    #    and gurukula_management must have descriptive query titles for q01..q60.
    with urllib.request.urlopen(f"{playground_server}/data/corpus.json", timeout=5) as resp:
        corpus = json.loads(resp.read().decode("utf-8"))

    for proj in corpus["projects"]:
        assert "//" not in proj["schema_ssql"], f"// comment in schema of {proj['id']}"
        assert "//" not in proj["seed_ssql"], f"// comment in seed of {proj['id']}"
        for q in proj["queries"]:
            assert "//" not in q["ssql"], f"// comment in query {proj['id']}/{q['id']}"
            assert "/*" not in q["ssql"], f"/* comment in query {proj['id']}/{q['id']}"

    gurukula_mgt = next(p for p in corpus["projects"] if p["id"] == "gurukula_management/SAMSQL")
    q01 = next(q for q in gurukula_mgt["queries"] if q["id"] == "q01")
    assert "List the names and designations of all teachers" in q01["title"]
    assert "Sanskrit:" not in q01["title"]


def test_compact_header_and_no_sanskrit_hyphens():
    # Zero Devanagari word hyphens in website copy or tutorial content
    dev_hyphen = re.compile(r"[\u0900-\u0965\u0970-\u097F]-[\u0900-\u0965\u0970-\u097F]")
    for rel in ("website/index.html", "website/app.js", "website/data/content.json"):
        text = (REPO_ROOT / rel).read_text(encoding="utf-8")
        found = dev_hyphen.findall(text)
        assert not found, f"Found Devanagari word hyphens in {rel}: {found}"

    # Compact single-line header with short runtime status and top-right actions
    index_html = (REPO_ROOT / "website" / "index.html").read_text(encoding="utf-8")
    app_js = (REPO_ROOT / "website" / "app.js").read_text(encoding="utf-8")
    styles_css = (REPO_ROOT / "website" / "styles.css").read_text(encoding="utf-8")
    assert "brand-subtitle" not in index_html
    assert "● WASM Ready" in app_js
    assert "margin-inline-start: auto;" in styles_css


def test_multi_theme_fonts_scaling_and_github_pages():
    index_html = (REPO_ROOT / "website" / "index.html").read_text(encoding="utf-8")
    styles_css = (REPO_ROOT / "website" / "styles.css").read_text(encoding="utf-8")

    # Declarative Appearance Popover & Quick Zoom Controls
    assert 'commandfor="appearance-popover"' in index_html
    assert 'command="toggle-popover"' in index_html
    assert 'id="btn-font-dec"' in index_html
    assert 'id="btn-font-inc"' in index_html

    # 5 Color Themes & 3 Font Pairings
    for theme in ("bhojpatra", "sphatika", "tamrapatta", "nisha", "tamala"):
        assert f'data-theme-choice="{theme}"' in index_html
    for font in ("sans", "serif", "mono"):
        assert f'data-font-choice="{font}"' in index_html

    assert "[popover]:is(:popover-open, .\\:popover-open)" in styles_css
    assert "@media (forced-colors: active)" in styles_css

    # GitHub Pages static readiness
    assert (REPO_ROOT / "website" / ".nojekyll").exists()
    assert (REPO_ROOT / ".github" / "workflows" / "deploy-pages.yml").exists()


def test_hero_simplicity_and_single_line_cqi_invariants():
    index_html = (REPO_ROOT / "website" / "index.html").read_text(encoding="utf-8")
    styles_css = (REPO_ROOT / "website" / "styles.css").read_text(encoding="utf-8")

    # 1. No misleading "Free Clause Order" claim or bloated hero chrome
    assert "Free Clause Order" not in index_html
    assert "hero-card-header" not in index_html
    assert "hero-card-divider" not in index_html
    assert "hero-metrics" not in index_html

    # 2. Container-query inline sizing (cqi) + nowrap invariants for h1 and 3 CTA buttons
    assert "<h1>Sanskrit Grammar for Relational Algebra</h1>" in index_html
    assert "container-type: inline-size;" in styles_css
    assert "flex-wrap: nowrap;" in styles_css
    assert "@supports (font-size: 1cqi)" in styles_css
    assert "font-size: 3.8cqi;" in styles_css
    assert "font-size: 1.78cqi;" in styles_css


def test_nav_scrollspy_underline_indicator():
    styles_css = (REPO_ROOT / "website" / "styles.css").read_text(encoding="utf-8")
    app_js = (REPO_ROOT / "website" / "app.js").read_text(encoding="utf-8")

    assert "scroll-target-group: auto;" in styles_css
    assert ".site-nav a:is(:target-current, .\\:target-current, [aria-current=\"true\"])" in styles_css
    assert "border-block-end-color: var(--brand-primary);" in styles_css
    assert "setupScrollSpy" in app_js
    assert "IntersectionObserver" in app_js


def test_no_parens_project_context_and_sql_tab_sync(playground_server):
    # 1. Zero *_parens.सं files on disk
    parens_files = list((REPO_ROOT / "examples").rglob("*_parens.सं"))
    assert not parens_files, f"Found leftover _parens.सं files on disk: {parens_files}"

    # 2. Verify corpus.json has 0 parens in query titles/SSQL, and rich context/objectives/sample_sql
    with urllib.request.urlopen(f"{playground_server}/data/corpus.json", timeout=5) as resp:
        corpus = json.loads(resp.read().decode("utf-8"))

    for proj in corpus["projects"]:
        assert proj.get("domain"), f"Missing domain for {proj['id']}"
        assert proj.get("difficulty"), f"Missing difficulty for {proj['id']}"
        assert proj.get("summary"), f"Missing summary for {proj['id']}"
        assert proj.get("sample_sql"), f"Missing sample_sql for {proj['id']}"
        for q in proj["queries"]:
            assert "parens" not in q["id"].lower(), f"Found parens query ID {proj['id']}/{q['id']}"
            assert "(" not in q["title"] and ")" not in q["title"], (
                f"Found parens in query title {proj['id']}/{q['id']}: {q['title']}"
            )
            assert "(" not in q["ssql"] and ")" not in q["ssql"], (
                f"Found parens in query SSQL {proj['id']}/{q['id']}: {q['ssql']}"
            )
            assert q.get("objective"), f"Missing objective for {proj['id']}/{q['id']}"

    app_js = (REPO_ROOT / "website" / "app.js").read_text(encoding="utf-8")
    assert "renderScenarioContextCard" in app_js
    assert "Project Context:" in app_js
    assert "Query Objective:" in app_js
    assert "sqlInputEditor.value =" in app_js


def test_selector_bar_vertical_dialect_and_pane_tab_discoverability():
    index_html = (REPO_ROOT / "website" / "index.html").read_text(encoding="utf-8")
    styles_css = (REPO_ROOT / "website" / "styles.css").read_text(encoding="utf-8")
    app_js = (REPO_ROOT / "website" / "app.js").read_text(encoding="utf-8")

    # 1. Vertical radio group for SQLite / PostgreSQL dialect
    assert 'id="dialect-radio-group"' in index_html
    assert 'id="dialect-radio-sqlite"' in index_html
    assert 'id="dialect-radio-postgres"' in index_html
    assert 'name="sql-dialect"' in index_html
    assert "grid-template-columns: minmax(0, 1fr) minmax(0, 1.15fr) auto;" in styles_css

    # 2. Project Context & Query Objective hosted in the selector grid outside #panel-ssql
    assert "editor-context-header" not in index_html
    assert "project-context-box" in app_js
    assert "query-objective-box" in app_js

    # 3. Equal-width segmented pane tabs & wrapping chip categories (no hidden horizontal tabs)
    assert "grid-template-columns: repeat(3, minmax(0, 1fr));" in styles_css
    assert "grid-template-columns: repeat(4, minmax(0, 1fr));" in styles_css
    assert "container-type: inline-size scroll-state;" in styles_css
    assert "btn-layout-stacked" in index_html

    # 4. Line-number gutters on all edit boxes + Generated SQL box, always-visible scrollbars, and stripped SQL header
    for gutter_id in (
        "ssql-gutter",
        "schema-gutter",
        "sql-input-gutter",
        "exercise-gutter",
        "generated-sql-gutter",
    ):
        assert f'id="{gutter_id}"' in index_html
        assert gutter_id in app_js

    assert "scrollbar-gutter: stable;" in styles_css
    assert "scrollbar-width: auto;" in styles_css
    assert "@media (prefers-contrast: more)" in styles_css
    assert "overflow-y: scroll;" in styles_css
    assert "stripGeneratedSqlHeader" in app_js
    assert "renderGeneratedSqlWithGutter" in app_js
    assert "highlightSqlLine" in app_js
    assert "highlightSqlSource" in app_js


def test_independent_review_fixes():
    index_html = (REPO_ROOT / "website" / "index.html").read_text(encoding="utf-8")
    styles_css = (REPO_ROOT / "website" / "styles.css").read_text(encoding="utf-8")
    app_js = (REPO_ROOT / "website" / "app.js").read_text(encoding="utf-8")
    content_data = json.loads((REPO_ROOT / "website" / "data" / "content.json").read_text(encoding="utf-8"))
    corpus_data = json.loads((REPO_ROOT / "website" / "data" / "corpus.json").read_text(encoding="utf-8"))

    # 1. Meta description, OG tags, favicon, footer GitHub + License links, and 8-row Kāraka table
    assert 'name="description"' in index_html
    assert 'property="og:title"' in index_html
    assert 'rel="icon"' in index_html
    assert "src/design/constitution.md" not in index_html
    assert "द्वितीया (2nd, Accusative)" in index_html
    assert "चतुर्थी (4th, Dative)" in index_html
    assert "Absolutive" in index_html
    assert "https://github.com/performance/SSQL" in index_html

    # 2. Constitution is SSQL-specific, hyphenated, and has no double parens
    const_text = json.dumps(content_data["constitution"], ensure_ascii=False)
    assert "((" not in const_text and "))" not in const_text
    assert "real-world" in const_text.lower()
    assert "/* ... */" in const_text

    # 3. All 10 tutorials have distinct chapter-matched exercises and Ch 2 starter is not pre-filled
    ex_titles = {c["exercise"]["title"] for c in content_data["tutorials"]}
    assert len(ex_titles) == 10
    ch2 = next(c for c in content_data["tutorials"] if c["id"] == "track-a-02")
    assert "आह्रियताम्" not in ch2["exercise"]["starter_ssql"]

    # 4. Downloads have no raw backticks, include full 64-char sha256_full and platform_note
    for dl in content_data["downloads"]:
        assert "`" not in dl["subtitle"]
        assert "`" not in dl["title"]
        assert len(dl.get("sha256_full", "")) == 64
        assert dl.get("platform_note")

    # 5. Clean project titles (no raw leading numbers like '01 Maven Toys Sales', disambiguated Restaurant projects)
    proj_titles = [p["title"] for p in corpus_data["projects"]]
    assert len(proj_titles) == len(set(proj_titles)), f"Duplicate project titles: {proj_titles}"
    for title in proj_titles:
        assert not title[0].isdigit(), f"Project title starts with digit: {title}"

    # 6. App.js & Styles.css fixes (code block newline wrapping, alert callouts, PostgreSQL simulation, white-space: pre)
    assert "\\n\\n@@CODE_BLOCK_" in app_js
    assert "Simulated via In-Browser SQLite" in app_js
    assert "DROP VIEW IF EXISTS" in app_js
    assert "white-space: pre;" in styles_css


def test_second_review_fixes(playground_server):
    index_html = (REPO_ROOT / "website" / "index.html").read_text(encoding="utf-8")
    styles_css = (REPO_ROOT / "website" / "styles.css").read_text(encoding="utf-8")
    app_js = (REPO_ROOT / "website" / "app.js").read_text(encoding="utf-8")
    content_data = json.loads((REPO_ROOT / "website" / "data" / "content.json").read_text(encoding="utf-8"))
    corpus_data = json.loads((REPO_ROOT / "website" / "data" / "corpus.json").read_text(encoding="utf-8"))

    # #3: Non-destructive schema application & Reset Default Schema button
    assert 'id="btn-reset-schema"' in index_html
    assert "applySchemaDdlNonDestructively" in app_js

    # #4 & #19: 0-row SELECT empty state vs DML affected row count + multi-statement batch rendering
    assert "stmt.getColumnNames()" in app_js
    assert "0 rows returned" in app_js
    assert "splitExecutableSqlStatements" in app_js

    # #5 & #11: Mobile tutorials sidebar not sticky at <=840px and scroll-margin-block-start: 8.25rem
    assert "position: static;" in styles_css
    assert "scroll-margin-block-start: 8.25rem;" in styles_css

    # #8: Tutorial filters run on the bookstore's real data, so every quoted
    # value a tutorial compares a column with must occur in that data. The
    # playground carries no hand-written copy of the seed: it is compiled
    # from seed.सं into corpus.json.
    assert re.search(r"INSERT INTO \w", app_js) is None
    bookstore = next(p for p in corpus_data["projects"] if p["id"] == "university_bookstore/SAMSQL")
    for ch in content_data["tutorials"]:
        for value in re.findall(r'"([^"]+)" इत्य\S+ सम\S+ (?:अस्ति|नास्ति)', ch["markdown"]):
            assert f"'{value}'" in bookstore["setup_sql"], f"{ch['id']} filters on {value!r}, which the bookstore data does not contain"
    for ch in content_data["tutorials"]:
        assert "सरासरी" not in ch["markdown"], f"Found loanword सरासरी in {ch['id']}"
        assert "सरासरी" not in ch["exercise"]["solution_ssql"], f"Found loanword सरासरी in {ch['id']} exercise"

    # #9 & #23: Persistence of completed exercises, active tutorial, project DB snapshots, and URL query params
    assert "ssql-completed-exercises" in app_js
    assert "projectDbSnapshots" in app_js
    assert "syncUrlQueryParams" in app_js
    assert "resolveTutorialMdHrefToChapId" in app_js

    # #16 & #20: Inline zoom reset button, skip-link, lang="sa", role="alert", and --label-1..--label-7
    assert 'id="btn-font-reset-inline"' in index_html
    assert 'class="skip-link"' in index_html
    assert 'lang="sa"' in index_html
    assert 'role="alert"' in index_html
    for i in range(1, 8):
        assert f"--label-{i}:" in styles_css

    # #18: Humanized query titles across all 38 projects (no raw underscores in query titles)
    for proj in corpus_data["projects"]:
        for q in proj["queries"]:
            assert "_" not in q["title"], f"Raw underscore in query title {proj['id']}/{q['id']}: {q['title']}"

    # #21: Downloads install commands use /usr/local/bin/ and include source build fallback
    for dl in content_data["downloads"]:
        if dl["id"] in ("compiler", "lsp"):
            assert "/usr/local/bin/" in dl["install_cmd"]
            assert dl.get("source_cmd")











