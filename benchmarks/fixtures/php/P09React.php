<?php

namespace Illuminate\Argotfix;

use React\EventLoop\Loop;

class P09React
{
    public function run(): void
    {
        Loop::addTimer(1.0, function () {});
    }
}
