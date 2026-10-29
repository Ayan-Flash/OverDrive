use super::report::GpuReportSection;

pub struct HtmlReport;

impl HtmlReport {
    pub fn generate(title: &str, date: &str, sections: &[GpuReportSection]) -> String {
        let mut body = String::new();
        body.push_str(&format!(
            "<h1>{}</h1>\n<p class=\"date\">Generated: {}</p>\n",
            Self::escape(title),
            Self::escape(date)
        ));

        for section in sections {
            body.push_str(&Self::section_html(section));
        }

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0a0a0f; color: #e0e0e0; padding: 32px; }}
  h1 {{ font-size: 24px; color: #f04747; margin-bottom: 4px; }}
  .date {{ color: #666; font-size: 12px; margin-bottom: 24px; }}
  .section {{ background: #14141f; border: 1px solid #1e1e2e; border-radius: 8px; padding: 20px; margin-bottom: 20px; }}
  .section h2 {{ font-size: 16px; color: #f04747; margin-bottom: 12px; }}
  .section p {{ color: #888; font-size: 12px; margin-bottom: 12px; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
  th {{ text-align: left; padding: 8px 12px; background: #1a1a2e; color: #a0a0b0; font-weight: 600; text-transform: uppercase; font-size: 11px; letter-spacing: 0.5px; border-bottom: 1px solid #2a2a3e; }}
  td {{ padding: 8px 12px; border-bottom: 1px solid #1a1a2e; }}
  tr:hover td {{ background: #1a1a2e; }}
  .metric {{ color: #e0e0e0; font-weight: 500; }}
  .value {{ font-family: 'JetBrains Mono', 'Consolas', monospace; text-align: right; }}
  .current {{ color: #fff; }}
  .min {{ color: #3498db; }}
  .max {{ color: #e74c3c; }}
  .avg {{ color: #2ecc71; }}
  .footer {{ text-align: center; color: #444; font-size: 11px; margin-top: 32px; }}
  .hot {{ color: #e74c3c !important; }}
</style>
</head>
<body>
{body}
<div class="footer">OverDrive — Performance Report</div>
</body>
</html>"#,
            title = Self::escape(title),
            body = body,
        )
    }

    fn section_html(section: &GpuReportSection) -> String {
        let mut rows = String::new();
        for m in &section.metrics {
            let is_hot = m.metric.contains("Temperature") && m.max > 80.0;
            let hot_class = if is_hot { r#" class="hot""# } else { "" };
            rows.push_str(&format!(
                "<tr>
  <td class=\"metric\">{}</td>
  <td class=\"value current\">{:.1}</td>
  <td class=\"value min\">{:.1}</td>
  <td class=\"value max\"{}>{:.1}</td>
  <td class=\"value avg\">{:.1}</td>
  <td class=\"value\">{}</td>
</tr>\n",
                Self::escape(&m.metric),
                m.current,
                m.min,
                hot_class,
                m.max,
                m.avg,
                Self::escape(&m.unit),
            ));
        }

        format!(
            r#"<div class="section">
<h2>{}</h2>
<p>Sampled over {} seconds &middot; {} data points</p>
<table>
<thead>
<tr><th>Metric</th><th>Current</th><th>Min</th><th>Max</th><th>Avg</th><th>Unit</th></tr>
</thead>
<tbody>
{}
</tbody>
</table>
</div>"#,
            Self::escape(&section.gpu_name),
            section.sampling_duration_secs,
            section.sample_count,
            rows,
        )
    }

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
}
