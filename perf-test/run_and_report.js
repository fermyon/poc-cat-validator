const { execSync } = require("child_process");
const fs = require("fs");
const PDFDocument = require("pdfkit");
const { ChartJSNodeCanvas } = require("chartjs-node-canvas");

// ------------------------------
// Configuration
// ------------------------------
const vusLevels = [1, 2, 8, 16, 32, 64, 128, 256, 512];
const duration = "60s";
const k6Script = "load_test.js";
const tasks = ["simple_success_times", "simple_fail_times", "kv_success_times", "kv_fail_country_times"];
const chartWidth = 500;
const chartHeight = 300;
const token = process.env.TOKEN || "SINGULAR_TOKEN_VALUE";
const simpleUrl = process.env.SIMPLE_URL || "http://localhost:3000/validate/simple";
const kvUrl = process.env.KV_URL || "http://localhost:3000/validate";

// ------------------------------
// Helper: Run k6 for a given VUs
// ------------------------------
function runK6(vus) {
    console.log(`\nRunning k6 with VUs=${vus}...`);
    const filename = `results_vus_${vus}.json`;

    execSync(`k6 run ${k6Script}`, {
        stdio: "inherit",
        env: {
            ...process.env,
            TOKEN: token,
            SIMPLE_URL: simpleUrl,
            KV_URL: kvUrl,
            VUS: vus.toString(),
            DURATION: duration
        }
    });

    if (!fs.existsSync(filename)) throw new Error(`Expected JSON file ${filename} not found.`);
    console.log(`Completed VUs=${vus}`);
    return filename;
}

// ------------------------------
// Helper: safely get metric value
// ------------------------------
function getMetricValue(metric, field) {
    return metric && metric.values && metric.values[field] !== undefined
        ? metric.values[field].toFixed(2)
        : "N/A";
}

// ------------------------------
// Helper: generate chart buffer
// ------------------------------
async function generateLineChart(title, data, labels) {
    const chartJSNodeCanvas = new ChartJSNodeCanvas({ width: chartWidth, height: chartHeight });
    const config = {
        type: "line",
        data: {
            labels,
            datasets: [{ label: title, data, borderColor: "blue", fill: false, tension: 0.2 }]
        },
        options: {
            responsive: false,
            plugins: { legend: { display: true } },
            scales: {
                y: { beginAtZero: true, title: { display: true, text: "ms (p95 latency)" } },
                x: { title: { display: true, text: "Concurrency (VUs)" } }
            }
        }
    };
    return await chartJSNodeCanvas.renderToBuffer(config);
}

// ------------------------------
// Main function
// ------------------------------
(async () => {
    const pdfDoc = new PDFDocument({ autoFirstPage: false });
    const pdfPath = "load_test_report.pdf";
    pdfDoc.pipe(fs.createWriteStream(pdfPath));

    const resultsByVU = {};

    // 1️⃣ Run k6 once per VU level
    for (const vus of vusLevels) {
        try {
            const jsonFile = runK6(vus);
            const raw = fs.readFileSync(jsonFile);
            resultsByVU[vus] = JSON.parse(raw);
        } catch (err) {
            console.warn(err.message);
        }
    }

    // 2️⃣ Generate PDF by looping over tasks
    for (const task of tasks) {
        pdfDoc.addPage();
        pdfDoc.fontSize(16).text(`Task: ${task}`, { underline: true });

        pdfDoc.moveDown(0.5);
        pdfDoc.font("Courier").fontSize(12).text("Summary", { underline: true });
        pdfDoc.moveDown(0.5);
        pdfDoc.text("VUs  Min     Median  Avg     P95     Max");

        const p95Values = [];
        const validVUs = [];

        for (const vus of vusLevels) {
            const metrics = resultsByVU[vus];
            if (!metrics || !metrics[task]) continue;

            const metric = metrics[task];
            const min = getMetricValue(metric, "min");
            const max = getMetricValue(metric, "max");
            const med = getMetricValue(metric, "med");
            const avg = getMetricValue(metric, "avg");
            const p95 = getMetricValue(metric, "p(95)");

            if (p95 !== "N/A") {
                p95Values.push(parseFloat(p95));
                validVUs.push(vus);
            }

            pdfDoc.text(
                `${vus.toString().padEnd(4)} ${min.toString().padEnd(7)} ${med.toString().padEnd(7)} ${avg.toString().padEnd(7)} ${p95.toString().padEnd(7)} ${max.toString().padEnd(7)}`
            );
        }

        pdfDoc.font("Helvetica");

        if (p95Values.length > 0) {
            const buffer = await generateLineChart(`${task} - p95 vs Concurrency`, p95Values, validVUs);
            pdfDoc.image(buffer, { width: 450, align: "center" });
        }

        pdfDoc.moveDown(2);
    }

    pdfDoc.end();
    console.log(`\nPDF report generated: ${pdfPath}`);
})();
