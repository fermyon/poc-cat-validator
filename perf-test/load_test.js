import http from "k6/http";
import { sleep, check } from "k6";
import { Trend, Counter, Rate } from "k6/metrics";

// ------------------------------
// Settings / Constants
// ------------------------------
export const Settings = {
    TOKEN: __ENV.TOKEN || "SINGULAR_TOKEN_VALUE",
    SIMPLE_URL: __ENV.SIMPLE_URL || "http://localhost:3000/validate/simple",
    KV_URL: __ENV.KV_URL || "http://localhost:3000/validate",
    STREAM_M3U8: "https://my-streaming.api/media/foo.m3u8",
    STREAM_MP4: "https://my-streaming.api/media/foo.mp4"
};

// ------------------------------
// k6 Options
// ------------------------------
export const options = {
    vus: __ENV.VUS ? parseInt(__ENV.VUS) : 5,
    duration: __ENV.DURATION || "10s",
    thresholds: {
        "checks": ["rate>0.99"],   // fail run if validation fails
    }
};

// ------------------------------
// Custom metrics
// ------------------------------
export let simple_success_times = new Trend("simple_success_times", true);
export let simple_fail_times = new Trend("simple_fail_times", true);
export let kv_success_times = new Trend("kv_success_times", true);
export let kv_fail_country_times = new Trend("kv_fail_country_times", true);

export let success_count = new Counter("success_count");
export let fail_count = new Counter("fail_count");

export let http_200_rate = new Rate("http_200_rate");
export let http_403_rate = new Rate("http_403_rate");

// ------------------------------
// Default function
// ------------------------------
export default function () {

    const fullPayload = (url, country = "DE") => JSON.stringify({
        token: Settings.TOKEN,
        url: url,
        method: "GET",
        issuer: "http://issuer.local",
        headers: {
            "User-Agent": "Apple Mozilla Edge",
            "X-FWF-Custom-Header": "Lorem"
        },
        validate_not_before: true,
        validate_expiration: true,
        audience: "streaming-api-1",
        client_ip: "46.165.180.81",
        country: country
    });

    const headers = { "Content-Type": "application/json" };

    // simple_success (expect 200)
    let res1 = http.post(Settings.SIMPLE_URL, fullPayload(Settings.STREAM_M3U8), { headers });
    simple_success_times.add(res1.timings.duration);
    check(res1, { "simple_success is 200": (r) => r.status === 200 });
    http_200_rate.add(res1.status === 200);

    // simple_fail (expect 403)
    let res2 = http.post(Settings.SIMPLE_URL, fullPayload(Settings.STREAM_MP4), { headers });
    simple_fail_times.add(res2.timings.duration);
    check(res2, { "simple_fail is 403": (r) => r.status === 403 });
    http_403_rate.add(res2.status === 403);

    // kv_success (expect 200)
    let res3 = http.post(Settings.KV_URL, fullPayload(Settings.STREAM_M3U8), { headers });
    kv_success_times.add(res3.timings.duration);
    check(res3, { "kv_success is 200": (r) => r.status === 200 });
    http_200_rate.add(res3.status === 200);

    // kv_fail_country (expect 403) - swap country to US
    let res4 = http.post(Settings.KV_URL, fullPayload(Settings.STREAM_M3U8, "US"), { headers });
    kv_fail_country_times.add(res4.timings.duration);
    check(res4, { "kv_fail_country is 403": (r) => r.status === 403 });
    http_403_rate.add(res4.status === 403);

    sleep(1);
}

// ------------------------------
// JSON Summary with RPS
// ------------------------------
export function handleSummary(data) {
    const filename = `results_vus_${options.vus}.json`;

    const metrics = {
        ...data.metrics,
        rps: data.metrics.http_reqs.rate, // RPS here
        checks: data.metrics.checks
    };

    return {
        [filename]: JSON.stringify(metrics, null, 2)
    };
}
