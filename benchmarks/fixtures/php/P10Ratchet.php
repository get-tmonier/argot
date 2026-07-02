<?php

namespace Illuminate\Argotfix;

use Ratchet\Server\IoServer;

class P10Ratchet
{
    public function serve(IoServer $server): void
    {
        $server->run();
    }
}
