#!/usr/bin/env python3
# coding: utf-8

# In[71]:

from sexpdata import loads, dumps, Symbol


# In[72]:

import sys
with open(sys.argv[1]) as dbf:
    db = dbf.read()


# In[73]:

ldb = loads(db, false="#f")
#ldb


# In[74]:

def do_val(val):
    if len(val) == 1:
        val = val[0]
    elif isinstance(val, list):
        if len(val) > 0:
            if isinstance(val[0], Symbol):
                if val[0] == Symbol('.'):
                    val = do_val(val[1:])
                else:
                    val = to_dict(val)
            else:
                if isinstance(val[0][0], Symbol):
                    val = to_dict(val)
                else:
                    val = [to_dict(v) for v in val]
    return val

def replace(a):
    rs = {
        "cover": "poster_path",
        "total": "total_eps",
        "curr": "current_ep",
        "items": "shows",
    }
    if a in rs:
        return rs[a]
    return a

def to_dict(ls):
    d = {}
    for l in ls:
        #print("working on", l)
        key = replace(l[0].tosexp())
        val = do_val(l[1:])
    
        d[key] = val
        #print("output", val)
    return d

d = to_dict(ldb)
d['settings'] = {"player": "mpv --fullscreen", "path": "", "autoplay": False}
import json
print(json.dumps(d))


# In[ ]:



