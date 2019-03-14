prefix := rel bare
suffix := ext noext slash

expand_name = \
	file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-default.js \
	file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-browser.js \
	$(if $(filter noext,$(fs) $(ts)), \
		file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-default.json \
		file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-browser.json,) \
	$(if $(filter slash,$(fs) $(ts)), \
		file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-default/index.js \
		file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-browser/index.js,) \
	$(if $(filter bare,$(fp) $(tp)), \
		node_modules/file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-browser.js \
		$(if $(filter noext,$(fs) $(ts)), \
			node_modules/file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-browser.json,) \
		$(if $(filter slash,$(fs) $(ts)), \
			node_modules/file-from-$(fp)-$(fs)-to-$(tp)-$(ts)-browser/index.js,),)
files := $(foreach fp,$(prefix),\
	$(foreach fs,$(suffix),\
	$(foreach tp,$(prefix),\
	$(foreach ts,$(suffix),$(expand_name)))))

generate: $(files)

%.js:
	@mkdir -p $$(dirname ./$@)
	echo 'module.exports = __filename' >$@
%.json:
	@mkdir -p $$(dirname ./$@)
	echo '{}' >$@

.PHONY: generate
