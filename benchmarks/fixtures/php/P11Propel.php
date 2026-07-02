<?php

namespace Illuminate\Argotfix;

use Propel\Runtime\Propel;

class P11Propel
{
    public function conn(string $name)
    {
        return Propel::getConnection($name);
    }
}
