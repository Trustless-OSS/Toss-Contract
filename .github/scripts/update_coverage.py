#!/usr/bin/env python3
import re
import sys
import urllib.parse
from html import unescape



def extract_percentage(text):
    # Try several heuristics to find a coverage percentage
    patterns = [r'coverage[:\s]*([0-9]+\.?[0-9]*)%', r'([0-9]+\.?[0-9]*)%\s*coverage', r'([0-9]+\.?[0-9]*)%']
    for p in patterns:
        m = re.search(p, text, re.IGNORECASE)
        if m:
            try:
                return float(m.group(1))
            except ValueError:
                continue
    return None

def make_badge(percent):
    if percent is None:
        return '[![coverage](https://img.shields.io/badge/coverage-unknown-lightgrey.svg)](target/llvm-cov/index.html)'
    pct_str = f'{percent:.1f}'
    enc = urllib.parse.quote(f'{pct_str}%')
    if percent >= 90:
        color = 'brightgreen'
    elif percent >= 75:
        color = 'yellow'
    elif percent >= 50:
        color = 'orange'
    else:
        color = 'red'
    return f'[![coverage](https://img.shields.io/badge/coverage-{enc}-{color}.svg)](target/llvm-cov/index.html)'

def extract_table_from_index(html_path):
    try:
        with open(html_path, 'r', encoding='utf-8') as f:
            html = f.read()
    except FileNotFoundError:
        return None
    # Find links and nearby percentage values
    pattern = re.compile(r'<a[^>]+href=["\'](?P<href>[^"\']+)["\'][^>]*>(?P<name>[^<]+)</a>.*?(?P<pct>[0-9]+\.?[0-9]*)\s*%', re.IGNORECASE | re.DOTALL)
    matches = pattern.findall(html)
    if not matches:
        return None
    rows = []
    for href, name, pct in matches:
        name = unescape(name.strip())
        href = href.strip()
        pctf = float(pct)
        rows.append((name, pctf, href))
    # Build markdown table
    md = []
    md.append('| File | Coverage | Report |')
    md.append('| --- | ---: | --- |')
    base = 'target/llvm-cov/'
    for name, pct, href in rows:
        link = base + href
        md.append(f'| {name} | {pct:.1f}% | [view]({link}) |')
    return '\n'.join(md)


def update_readme(readme_path, badge_markdown, table_markdown=None):
    badge_start = '<!-- COVERAGE_BADGE_START -->'
    badge_end = '<!-- COVERAGE_BADGE_END -->'
    table_start = '<!-- COVERAGE_START -->'
    table_end = '<!-- COVERAGE_END -->'
    with open(readme_path, 'r', encoding='utf-8') as f:
        rd = f.read()

    # Update badge block
    if badge_start in rd and badge_end in rd:
        before, rest = rd.split(badge_start, 1)
        _, after = rest.split(badge_end, 1)
        rd = before + badge_start + '\n' + badge_markdown + '\n' + badge_end + after
    else:
        rd = badge_start + '\n' + badge_markdown + '\n' + badge_end + '\n\n' + rd

    # Update table block if provided
    if table_markdown is not None:
        if table_start in rd and table_end in rd:
            before, rest = rd.split(table_start, 1)
            _, after = rest.split(table_end, 1)
            rd = before + table_start + '\n' + table_markdown + '\n' + table_end + after
        else:
            rd = rd + '\n\n' + table_start + '\n' + table_markdown + '\n' + table_end + '\n'

    with open(readme_path, 'w', encoding='utf-8') as f:
        f.write(rd)

def main():
    cov_file = sys.argv[1] if len(sys.argv) > 1 else 'coverage-output.txt'
    readme = sys.argv[2] if len(sys.argv) > 2 else 'README.md'
    try:
        with open(cov_file, 'r', encoding='utf-8') as f:
            out = f.read()
    except FileNotFoundError:
        out = ''
    percent = extract_percentage(out)
    badge = make_badge(percent)
    # Try to extract per-file table from generated HTML
    table = extract_table_from_index('target/llvm-cov/index.html')
    update_readme(readme, badge, table)
    print('Wrote badge:', badge)
    if table:
        print('Wrote coverage table with', table.count('\n'), 'lines')
    else:
        print('No coverage table found')

if __name__ == '__main__':
    main()
