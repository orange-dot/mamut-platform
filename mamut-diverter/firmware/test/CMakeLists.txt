# Unity test framework (expected in test/unity/)
add_executable(test_diverter test_diverter.c)
target_link_libraries(test_diverter mamut_firmware)
add_test(NAME test_diverter COMMAND test_diverter)
