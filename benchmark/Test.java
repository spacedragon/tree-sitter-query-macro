package com.test;

public static class Test<T> extends Object<A> implements Function<Long, Long>, Runnable {

    private int field1 = 1 + 1;

    public Test() {
        this.field1 = 2;
    }

    @Override
    public Long apply(Long aLong, String text) {
        return aLong + field1;
    }
}