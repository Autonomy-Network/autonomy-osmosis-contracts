import * as LocalOsmosis from './localosmosis';

//----------------------------------------------------------------------------------------
// Test-suite
//----------------------------------------------------------------------------------------
(async () => {
	const mode = process.env.npm_config_mode || "";
	switch (mode) {

		/* -   Osmosis local network    -  */

		case "localosmosis_setup_common":
			await LocalOsmosis.startSetupCommon();
			break;

		case "localosmosis_tests_wrapperosmosis":
			await LocalOsmosis.startTestsWrapperOsmosis();
			break;

		default:
			console.log("Invalid command");
			break;
	}
})();
