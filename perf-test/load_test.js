import http from "k6/http";
import { sleep } from "k6";
import { Trend } from "k6/metrics";

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
    duration: __ENV.DURATION || "10s"
};

// ------------------------------
// Custom metrics (store all values)
// ------------------------------
export let simple_success_times = new Trend("simple_success_times", true);
export let simple_fail_times = new Trend("simple_fail_times", true);
export let kv_success_times = new Trend("kv_success_times", true);
export let kv_fail_country_times = new Trend("kv_fail_country_times", true);

// ------------------------------
// Default k6 function
// ------------------------------
export default function () {
    // simple_success
    let payload = JSON.stringify({ token: Settings.TOKEN, url: Settings.STREAM_M3U8 });
    simple_success_times.add(http.post(Settings.SIMPLE_URL, payload).timings.duration);

    // simple_fail
    payload = JSON.stringify({ token: Settings.TOKEN, url: Settings.STREAM_MP4 });
    simple_fail_times.add(http.post(Settings.SIMPLE_URL, payload).timings.duration);

    // kv_success
    payload = JSON.stringify({ token: Settings.TOKEN, url: Settings.STREAM_M3U8 });
    kv_success_times.add(http.post(Settings.KV_URL, payload).timings.duration);

    // kv_fail_country
    payload = JSON.stringify({ token: Settings.TOKEN, url: Settings.STREAM_M3U8 });
    kv_fail_country_times.add(http.post(Settings.KV_URL, payload).timings.duration);

    sleep(1);
}

// ------------------------------
// Handle summary - export JSON
// ------------------------------
export function handleSummary(data) {
    const filename = `results_vus_${options.vus}.json`;
    return {
        [filename]: JSON.stringify(data.metrics, null, 2)
    };
}
