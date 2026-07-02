package com.google.common.argotfix;

import io.reactivex.Observable;

public class J11Rxjava {
    public Observable<Integer> stream() {
        return Observable.just(1, 2, 3).map(x -> x * 2);
    }
}
