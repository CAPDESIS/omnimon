#!/bin/bash
# generate_nist_report.sh
# Generates an HTML report from Grype JSON output, mapped to NIST guidelines.

INPUT_FILE=$1
OUTPUT_FILE=$2

if [ -z "$INPUT_FILE" ] || [ -z "$OUTPUT_FILE" ]; then
    echo "Usage: $0 <grype-json-file> <output-html-file>"
    exit 1
fi

# Parse JSON using jq
CRITICAL=$(jq '.matches | map(select(.vulnerability.severity == "Critical")) | length' "$INPUT_FILE")
HIGH=$(jq '.matches | map(select(.vulnerability.severity == "High")) | length' "$INPUT_FILE")
MEDIUM=$(jq '.matches | map(select(.vulnerability.severity == "Medium")) | length' "$INPUT_FILE")
LOW=$(jq '.matches | map(select(.vulnerability.severity == "Low")) | length' "$INPUT_FILE")
TOTAL=$(($CRITICAL + $HIGH + $MEDIUM + $LOW))

# Determine overall posture
POSTURE_COLOR="#28a745"
POSTURE_TEXT="Healthy"
if [ "$CRITICAL" -gt 0 ]; then
    POSTURE_COLOR="#dc3545"
    POSTURE_TEXT="Critical Risk"
elif [ "$HIGH" -gt 0 ]; then
    POSTURE_COLOR="#ffc107"
    POSTURE_TEXT="Elevated Risk"
fi

cat <<EOF > "$OUTPUT_FILE"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>OmniMon System Security Status (NIST SP 800-53)</title>
    <style>
        body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; background: #f4f7f6; color: #333; margin: 0; padding: 20px; }
        .container { max-width: 900px; margin: auto; background: #fff; padding: 30px; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); }
        h1 { color: #0056b3; border-bottom: 2px solid #0056b3; padding-bottom: 10px; }
        .meta { font-size: 0.9em; color: #666; margin-bottom: 20px; }
        .posture-box { text-align: center; padding: 20px; border-radius: 6px; background-color: $POSTURE_COLOR; color: white; margin-bottom: 30px; }
        .posture-box h2 { margin: 0; font-size: 2em; }
        .chart-container { margin: 30px 0; }
        .bar-wrap { display: flex; align-items: center; margin: 10px 0; }
        .bar-label { width: 100px; font-weight: bold; }
        .bar-bg { flex-grow: 1; background: #e9ecef; height: 20px; border-radius: 10px; overflow: hidden; margin-right: 15px; }
        .bar-fill { height: 100%; transition: width 0.5s; }
        .bar-fill.critical { background: #dc3545; }
        .bar-fill.high { background: #fd7e14; }
        .bar-fill.medium { background: #ffc107; }
        .bar-fill.low { background: #17a2b8; }
        .vuln-list { border-collapse: collapse; width: 100%; }
        .vuln-list th, .vuln-list td { border: 1px solid #ddd; padding: 10px; text-align: left; }
        .vuln-list th { background: #f8f9fa; }
        .badge { padding: 4px 8px; border-radius: 4px; font-size: 0.85em; font-weight: bold; color: white; }
        .badge.critical { background: #dc3545; }
        .badge.high { background: #fd7e14; }
    </style>
</head>
<body>
    <div class="container">
        <h1>System Security Status</h1>
        <div class="meta">
            <strong>Date:</strong> $(date -u +"%Y-%m-%dT%H:%M:%SZ") <br>
            <strong>Tool:</strong> Grype Dependency Vulnerability Scanner <br>
            <strong>Framework:</strong> NIST Risk Management Framework (NIST SP 800-53)
        </div>

        <div class="posture-box">
            <p>Overall Security Posture</p>
            <h2>$POSTURE_TEXT</h2>
        </div>

        <h2>Vulnerability Breakdown</h2>
        <div class="chart-container">
            <div class="bar-wrap">
                <div class="bar-label">Critical</div>
                <div class="bar-bg"><div class="bar-fill critical" style="width: $(($TOTAL == 0 ? 0 : $CRITICAL * 100 / $TOTAL))%;"></div></div>
                <div>$CRITICAL</div>
            </div>
            <div class="bar-wrap">
                <div class="bar-label">High</div>
                <div class="bar-bg"><div class="bar-fill high" style="width: $(($TOTAL == 0 ? 0 : $HIGH * 100 / $TOTAL))%;"></div></div>
                <div>$HIGH</div>
            </div>
            <div class="bar-wrap">
                <div class="bar-label">Medium</div>
                <div class="bar-bg"><div class="bar-fill medium" style="width: $(($TOTAL == 0 ? 0 : $MEDIUM * 100 / $TOTAL))%;"></div></div>
                <div>$MEDIUM</div>
            </div>
            <div class="bar-wrap">
                <div class="bar-label">Low</div>
                <div class="bar-bg"><div class="bar-fill low" style="width: $(($TOTAL == 0 ? 0 : $LOW * 100 / $TOTAL))%;"></div></div>
                <div>$LOW</div>
            </div>
        </div>

        <h2>Detailed Vulnerabilities (Critical & High)</h2>
        <table class="vuln-list">
            <thead>
                <tr>
                    <th>Severity</th>
                    <th>CVE ID</th>
                    <th>Component</th>
                    <th>Description</th>
                </tr>
            </thead>
            <tbody>
EOF

# Append table rows
jq -r '.matches[] | select(.vulnerability.severity == "Critical" or .vulnerability.severity == "High") | "<tr><td><span class=\"badge " + (.vulnerability.severity | ascii_downcase) + "\">" + .vulnerability.severity + "</span></td><td>" + .vulnerability.id + "</td><td>" + .artifact.name + " (" + .artifact.version + ")</td><td>" + .vulnerability.description + "</td></tr>"' "$INPUT_FILE" >> "$OUTPUT_FILE"

cat <<EOF >> "$OUTPUT_FILE"
            </tbody>
        </table>

        <h2 style="margin-top: 40px;">Remediation & Mitigation (SI-2 Flaw Remediation)</h2>
        <p>All Critical and High vulnerabilities must be remediated or patched within 14 days of discovery. Continuous mitigation mechanisms are in place, and any changes failing CI code coverage (< 85%) will block automated merging to ensure code quality and integrity (SA-11).</p>
    </div>
</body>
</html>
EOF

echo "NIST Compliance Report (HTML) generated: $OUTPUT_FILE"

