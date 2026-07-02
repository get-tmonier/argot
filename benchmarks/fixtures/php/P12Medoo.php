<?php

namespace Illuminate\Argotfix;

use Medoo\Medoo;

class P12Medoo
{
    public function db(array $opts): Medoo
    {
        return new Medoo($opts);
    }
}
