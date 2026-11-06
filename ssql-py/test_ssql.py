"""Tests for the SSQL Python binding (`ssql`).

Run with: uv run --frozen pytest ssql-py/test_ssql.py
"""

import pytest
import ssql

BOOKS_SCHEMA = """
पुस्तकानि इति पटलम् "books" इति आख्या पुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
    प्राथमिकी क्रमिका सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    शीर्षकम् इति वाक्यम् "title" इति आख्या शीर्षक इति नपुंसकलिङ्गम् एकवचनम् च
    मूल्यम् इति वास्तविकः "price" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् च
  इति पटलम् ।
"""

DOC_WORDS_SCHEMA = """
लेखशब्दाः इति पटलम् "doc_words" इति आख्या लेखशब्द इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    लेखपरिचयः इति वाक्यम् "doc_uuid" इति आख्या लेखपरिचय इति पुंलिङ्गम् एकवचनम्
  इति पटलम् ।
"""


def sql_body(sql: str) -> str:
    """The generated SQL without header comments, whitespace-normalised."""
    return " ".join(" ".join(l for l in sql.splitlines() if not l.startswith("--")).split())


@pytest.fixture
def books():
    return ssql.Compiler(BOOKS_SCHEMA, "sqlite")


def test_basic_select(books):
    assert sql_body(books.compile("पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।")) == "SELECT t1.title FROM books t1;"


def test_compile_safe_reports_success_and_failure(books):
    ok = books.compile_safe("पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।")
    assert ok.success and ok.error is None and "SELECT" in ok.sql
    bad = books.compile_safe("पुस्तकेभ्यः शीर्षकम् ।")
    assert not bad.success and bad.sql is None
    assert "Missing action verb" in bad.error


def test_postgres_dialect():
    pg = ssql.Compiler(BOOKS_SCHEMA, "postgres")
    assert "price NUMERIC" in pg.compile(BOOKS_SCHEMA)


def test_external_parameter_becomes_placeholder():
    c = ssql.Compiler(DOC_WORDS_SCHEMA, "sqlite")
    sql = sql_body(c.compile(
        "लक्ष्यपरिचयः इति बाह्या सञ्ज्ञा । "
        "लेखशब्देभ्यः लेखपरिचयः लक्ष्यपरिचयः इत्यनेन समः अस्ति इति नियमेन लेखपरिचयस्य गणना आह्रियताम् ।"
    ))
    assert "COUNT(t1.doc_uuid)" in sql and "?" in sql


def test_count_and_select_all(books):
    assert "COUNT(*)" in books.compile("पुस्तकेभ्यः पङ्क्तीनाम् गणना आह्रियताम् ।")
    assert sql_body(books.compile("पुस्तकेभ्यः सर्वम् आह्रियताम् ।")) == "SELECT * FROM books t1;"


def test_explicit_field_access():
    c = ssql.Compiler(DOC_WORDS_SCHEMA, "sqlite")
    sql = sql_body(c.compile(
        'लेखशब्देभ्यः लेखशब्दस्य लेखपरिचयः "123" इत्यनेन समः अस्ति इति नियमेन लेखशब्दस्य लेखपरिचयः आह्रियताम् ।'
    ))
    assert "WHERE t1.doc_uuid = '123'" in sql


def test_errors_raise_compile_error_with_cause(books):
    with pytest.raises(ssql.CompileError) as err:
        books.compile("Invalid SSQL Garbage")
    # The full cause chain, not just the outermost context.
    assert "Lexer failed:" in str(err.value) and "Unexpected character" in str(err.value)


def test_calls_do_not_leak_ctes_into_later_calls(books):
    with_cte = (
        "मूल्यसारः इति आख्या मूल्यसार इति पुंलिङ्गम् एकवचनम् पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम् । "
        "मूल्यसारात् शीर्षकम् आह्रियताम् ।"
    )
    assert "WITH मूल्यसार AS" in books.compile(with_cte)
    # A later query that only names the CTE must not compile on the strength
    # of the earlier call (a fresh compiler rejects it too).
    with pytest.raises(ssql.CompileError):
        books.compile("मूल्यसारात् शीर्षकम् आह्रियताम् ।")
    with pytest.raises(ssql.CompileError):
        ssql.Compiler(BOOKS_SCHEMA, "sqlite").compile("मूल्यसारात् शीर्षकम् आह्रियताम् ।")
