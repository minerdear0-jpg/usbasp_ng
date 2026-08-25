Per-FUNC contract is in `avrdude/spec.yaml` plus `avrdude/test_contract.py`.

Named cases (all asserted via the spec + header parse, not live USB):

- getcapabilities
- connect
- setispsck
- transmit
- readflash
- writeflash
- readeeprom
- writeeeprom
- setlongaddress
- tpi
