EM_ROOT := $(HOME)/.local/git/eaglemode-0.96.4
CXX     := g++
CXXFLAGS := -O2 -std=c++17 -I$(EM_ROOT)/include -I$(EM_ROOT)
LDFLAGS  := -L$(EM_ROOT)/lib -Wl,-rpath,$(EM_ROOT)/lib -lemCore -lemTestPanel $(EM_ROOT)/obj/emTkTestPanel.o -lm -lpthread -ldl

gen_golden: gen_golden.cpp tktest_factory.o golden_format.h
	$(CXX) $(CXXFLAGS) -o $@ gen_golden.cpp tktest_factory.o $(LDFLAGS)

tktest_factory.o: tktest_factory.cpp
	$(CXX) $(CXXFLAGS) -c -o $@ $<

run: gen_golden
	cd ../../.. && EM_DIR=$(EM_ROOT) LD_LIBRARY_PATH=$(EM_ROOT)/lib:$$LD_LIBRARY_PATH tests/golden/gen/gen_golden

clean:
	rm -f gen_golden tktest_factory.o

.PHONY: run clean
