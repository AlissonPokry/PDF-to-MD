from pdftomd.worker import (
    Line,
    Page,
    Span,
    detect_heading_level,
    merge_paragraph_lines,
    render_markdown,
    spans_to_lines,
)


def test_merge_paragraph_lines_removes_hyphenated_line_breaks():
    merged = merge_paragraph_lines(["real-", "mente satisfeitos"])

    assert merged == "realmente satisfeitos"


def test_merge_paragraph_lines_keeps_normal_word_spacing():
    merged = merge_paragraph_lines(["Ordem Paranormal", "com diversos conteudos"])

    assert merged == "Ordem Paranormal com diversos conteudos"


def test_detect_heading_level_uses_large_type_for_title():
    line = Line("MISSOES COM TRANSTORNADOS", 85, 686, 520, 714, 28, True)

    assert detect_heading_level(line, body_size=11) == 1


def test_detect_heading_level_uses_medium_type_for_section():
    line = Line("CHORO DO ASFALTO", 302, 648, 500, 665, 17, True)

    assert detect_heading_level(line, body_size=11) == 2


def test_render_markdown_flushes_paragraph_before_heading():
    page = Page(
        number=40,
        width=595,
        height=842,
        lines=(
            Line("A violencia dos Transtornados e", 94, 664, 250, 675, 11),
            Line("uma ferramenta poderosa de horror", 94, 649, 250, 660, 11),
            Line("CHORO DO ASFALTO", 302, 648, 500, 665, 17, True),
            Line("Moradores de um bairro periferico", 302, 629, 500, 640, 11),
        ),
    )

    markdown = render_markdown([page])

    assert "A violencia dos Transtornados e uma ferramenta poderosa de horror\n\n## CHORO DO ASFALTO" in markdown


def test_spans_to_lines_keeps_same_row_columns_separate():
    spans = [
        Span("left column text", 68, 100, 245, 112, 11),
        Span("right column text", 302, 100, 480, 112, 11),
    ]

    lines = spans_to_lines(spans, page_width=595)

    assert [line.text for line in lines] == ["left column text", "right column text"]


def test_spans_to_lines_splits_when_left_text_slightly_crosses_boundary():
    spans = [
        Span("left spillover", 160, 55, 282, 67, 11),
        Span("right column", 300, 55, 420, 67, 11),
    ]

    lines = spans_to_lines(spans, page_width=581)

    assert [line.text for line in lines] == ["left spillover", "right column"]


def test_render_markdown_breaks_paragraphs_on_vertical_gap():
    page = Page(
        number=9,
        width=595,
        height=842,
        lines=(
            Line("First paragraph line one", 68, 100, 250, 112, 11),
            Line("line two", 68, 116, 250, 128, 11),
            Line("Second paragraph", 91, 140, 250, 152, 11),
        ),
    )

    markdown = render_markdown([page])

    assert "First paragraph line one line two\n\nSecond paragraph" in markdown


def test_render_markdown_breaks_paragraphs_between_columns():
    page = Page(
        number=6,
        width=595,
        height=842,
        lines=(
            Line("Left column", 68, 100, 250, 112, 11),
            Line("Right column", 302, 90, 500, 102, 11),
        ),
    )

    markdown = render_markdown([page])

    assert markdown == "Left column\n\nRight column\n"
