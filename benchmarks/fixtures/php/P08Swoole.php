<?php

namespace Illuminate\Argotfix;

use Swoole\Http\Server;

class P08Swoole
{
    public function server(): Server
    {
        return new Server("0.0.0.0", 9501);
    }
}
