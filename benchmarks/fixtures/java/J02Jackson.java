package com.google.common.argotfix;

import com.fasterxml.jackson.databind.ObjectMapper;

public class J02Jackson {
    public String toJson(Object o) throws Exception {
        return new ObjectMapper().writeValueAsString(o);
    }
}
