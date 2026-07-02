<?php

namespace Illuminate\Argotfix;

use Cake\ORM\TableRegistry;

class P04Cake
{
    public function articles()
    {
        return TableRegistry::getTableLocator()->get("Articles");
    }
}
