package com.google.common.argotfix;

import retrofit2.Retrofit;
import retrofit2.converter.gson.GsonConverterFactory;

public class J10Retrofit {
    public Retrofit build(String base) {
        return new Retrofit.Builder().baseUrl(base)
            .addConverterFactory(GsonConverterFactory.create()).build();
    }
}
