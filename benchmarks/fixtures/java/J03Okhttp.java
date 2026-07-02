package com.google.common.argotfix;

import okhttp3.OkHttpClient;
import okhttp3.Request;

public class J03Okhttp {
    private final OkHttpClient client = new OkHttpClient();
    public Request build(String url) { return new Request.Builder().url(url).build(); }
}
