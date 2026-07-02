package com.google.common.argotfix;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class J06Slf4j {
    private static final Logger log = LoggerFactory.getLogger(J06Slf4j.class);
    public void run() { log.info("running"); }
}
